#![allow(unstable)]
extern crate sdl2;
#[macro_use] extern crate log;

extern crate actors;
extern crate specs;
extern crate conf;
extern crate input;
extern crate ai;
extern crate network;

use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::cmp::min;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::collections::RingBuf;
use std::thread::Thread;
use std::ops::Deref;
use std::io::IoErrorKind;

use actors::*;
use specs::*;
use conf::*;
use input::*;
use ai::*;

// ---------------------------------------------------------------------
// Generic client handle and utilities

pub trait ClientSend {
    /// `false` if we should stop.
    fn send_input(&mut self, input: Input) -> bool;
}

pub trait ClientRecv {
    /// `None` if we should stop.
    fn recv_game(&mut self) -> Option<PlayerGame>;
}

pub fn attach_ai<A: Ai, S: ClientSend, R: ClientRecv>(send: &mut S, recv: &mut R, ai: &Ai) {
    loop {
        match recv.recv_game() {
            None => break,
            Some(player_game) => {
                let input = ai.move_(&player_game);
                if !send.send_input(input) { break };
            }
        }
    }
}

pub fn attach_sdl<S: ClientSend + Send + Clone, R: ClientRecv, F: Fn(PlayerGame)>(send: &S, recv: &mut R, on_game_update: F) {
    let (quit_tx, quit_rx) = channel();
    let mut worker_send = send.clone();

    // Thread sending inputs
    let _ = Thread::spawn(move || {
        // Send input every 5ms
        let mut input = Input::new();

        loop {
            let new_input = input.process_events();
            if new_input.quit {
                let _ = quit_tx.send(());
                break
            }
            if new_input != input {
                input = new_input;
                let alive = worker_send.send_input(input);
                if !alive { break };
            }
            sdl2::timer::delay(5);
        }
    });

    // Get the game and draw
    loop {
        let quit = quit_rx.try_recv();
        match quit {
            Ok(()) => break,
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        match recv.recv_game() {
            None => break,
            Some(game) => on_game_update(game),
        };
    };
}

// ---------------------------------------------------------------------
// Server

const SERVER_GAMES: usize = 32;

pub struct Server {
    spec: Arc<GameSpec>,
    games: Arc<Mutex<RingBuf<Game>>>,
    clients: Arc<Mutex<HashMap<ActorId, Sender<Arc<Game>>>>>,
    cmds_tx: Sender<(ActorId, Input)>,
    cmds_rx: Receiver<(ActorId, Input)>,
}

impl Server {
    pub fn new(spec: Arc<GameSpec>, game: Game) -> Server {
        let (cmds_tx, cmds_rx) = channel();
        let mut games = RingBuf::with_capacity(SERVER_GAMES);
        games.push_front(game);
        Server{
            spec: spec,
            games: Arc::new(Mutex::new(games)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            cmds_tx: cmds_tx,
            cmds_rx: cmds_rx,
        }
    }

    pub fn join_handle(&self) -> JoinHandle {
        JoinHandle{
            spec: self.spec.clone(),
            games: self.games.clone(),
            clients: self.clients.clone(),
            cmds_tx: self.cmds_tx.clone(),
        }
    }

    fn remove_player(&self, player: ActorId) {
        // They might be both have been removed already
        {
            let mut clients = self.clients.lock().unwrap();
            let _ = clients.remove(&player);
        };
        {
            let mut games = self.games.lock().unwrap();
            let mut game = games.front_mut().unwrap();
            let _ = game.actors.remove(player);
        };
        info!("Player {} left the game -- disconnected when sending", player);
    }
    
    fn broadcast(&self, game: Arc<Game>) {
        // When a client is disconnected, clean it up
        let mut dead: Vec<ActorId> = Vec::new();
        {
            // Lock clients
            let clients = self.clients.lock().unwrap();
            for (actor_id, tx) in clients.iter()  {
                let mb_err = tx.send(game.clone());
                if mb_err.is_err() {
                    dead.push(*actor_id);
                } else {
                    debug!("Game sent to {}", actor_id);
                }
            };
            // Unlock clients
        }
        for actor_id in dead.iter() {
            self.remove_player(*actor_id);
        }
    }

    fn prepare_inputs(&self) -> Option<Vec<PlayerInput>> {
        let mut cmds: Vec<PlayerInput> = Vec::new();
        loop {
            match self.cmds_rx.try_recv() {
                Ok((player, x)) => {
                    debug!("Got input from player {}", player);
                    cmds.push(PlayerInput{
                        player: player,
                        input: x,
                    })
                },
                Err(TryRecvError::Empty) => return Some(cmds),
                Err(TryRecvError::Disconnected) => return None,
            }
        }
    }

    pub fn run(&self) {
        let wait_ms = (TIME_STEP * 1000.) as usize;

        loop {
            let time_begin = sdl2::get_ticks() as usize;

            match self.prepare_inputs() {
                None => break,
                Some(inputs) => {
                    let game = {
                        let mut games = self.games.lock().unwrap();
                        let new_game = games.front().unwrap().advance(self.spec.deref(), &inputs, TIME_STEP);
                        if games.len() >= SERVER_GAMES {
                            games.pop_back().unwrap();
                        };
                        games.push_front(new_game.clone());
                        new_game
                    };
                    self.broadcast(Arc::new(game));
                    let time_end = sdl2::get_ticks() as usize;
                    sdl2::timer::delay(wait_ms - min(wait_ms, time_end - time_begin));
                },
            }
        }
    }
}

// We need this to have a clonable joiner
#[derive(Clone)]
pub struct JoinHandle {
    spec: Arc<GameSpec>,
    games: Arc<Mutex<RingBuf<Game>>>,
    clients: Arc<Mutex<HashMap<ActorId, Sender<Arc<Game>>>>>,
    cmds_tx: Sender<(ActorId, Input)>,
}

impl JoinHandle {
    pub fn join(&self) -> (ActorId, ServerClientSend, ServerClientRecv) {
        let player = {
            let mut games = self.games.lock().unwrap();
            games.front_mut().unwrap().add_ship(self.spec.deref())
        };
        let rx = {
            let mut clients = self.clients.lock().unwrap();
            let (tx, rx) = channel();
            clients.insert(player, tx);
            rx
        };
        info!("Player {} joined.", player);
        (player,
         ServerClientSend{player: player, sender: self.cmds_tx.clone()},
         ServerClientRecv{player: player, receiver: rx})
    }
}

// ---------------------------------------------------------------------
// ServerClient

#[derive(Clone)]
pub struct ServerClientSend {
    player: ActorId,
    sender: Sender<(ActorId, Input)>,
}

impl ClientSend for ServerClientSend {
    fn send_input(&mut self, input: Input) -> bool {
        let send_res = self.sender.send((self.player, input));
        if send_res.is_err() { return false };
        true
    }
}

pub struct ServerClientRecv {
    player: ActorId,
    receiver: Receiver<Arc<Game>>,
}

impl ClientRecv for ServerClientRecv {
    fn recv_game(&mut self) -> Option<PlayerGame> {
        let recv_res = self.receiver.recv();
        match recv_res {
            Err(_) => None,
            Ok(game) => Some(PlayerGame{
                player: self.player,
                game: game
            }),
        }
    }
}

// ---------------------------------------------------------------------
// Network `ClientHandle`

impl ClientSend for network::ClientHandle {
    fn send_input(&mut self, input: Input) -> bool {
        loop {
            let send_res = self.send(&input);
            match send_res {
                Err(err) => match err.kind {
                    IoErrorKind::Closed => return false,
                    _ => warn!("Got unexpected error {}, continuing", err),
                },
                Ok(()) => return true,
            }
        }
    }
}

impl ClientRecv for network::ClientHandle {
    fn recv_game(&mut self) -> Option<PlayerGame> {
        loop {
            self.set_timeout(Some(5));
            let recv_res = self.recv();
            match recv_res {
                Err(err) => match err.kind {
                    IoErrorKind::Closed => return None,
                    IoErrorKind::TimedOut => (),
                    _ => warn!("Got unexpected error {}, continuing", err),
                },
                Ok(game) => return Some(game),
            }
        }
    }
}
