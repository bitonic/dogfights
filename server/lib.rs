extern crate sdl2;
#[macro_use] extern crate log;

extern crate actors;
extern crate specs;
extern crate conf;
extern crate input;

use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError, SendError};
use std::cmp::min;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::collections::RingBuf;

use actors::*;
use specs::*;
use conf::*;
use input::*;

const SERVER_GAMES: usize = 32;

pub struct Server {
    pub spec: GameSpec,
    pub games: Arc<Mutex<RingBuf<Game>>>,
    pub clients: Arc<Mutex<HashMap<ActorId, Sender<Arc<Game>>>>>,
    pub cmds_tx: Sender<(ActorId, Input)>,
    pub cmds_rx: Receiver<(ActorId, Input)>,
}

#[derive(Clone)]
pub struct ServerHandle {
    pub spec: GameSpec,
    // Invariant: the `RingBuf` is not empty
    pub games: Arc<Mutex<RingBuf<Game>>>,
    pub cmds: Sender<(ActorId, Input)>,
    pub clients: Arc<Mutex<HashMap<ActorId, Sender<Arc<Game>>>>>,
}

impl ServerHandle {
    pub fn join(&self) -> (ActorId, Receiver<Arc<Game>>) {
        let player = {
            let mut games = self.games.lock().unwrap();
            games.front_mut().unwrap().add_ship(&self.spec)
        };
        let rx = {
            let mut clients = self.clients.lock().unwrap();
            let (tx, rx) = channel();
            clients.insert(player, tx);
            rx
        };
        info!("Player {} joined.", player);
        (player, rx)
    }

    pub fn send(&self, player: ActorId, cmd: Input) -> Result<(), SendError<(ActorId, Input)>> {
        debug!("Player {} about to send input", player);
        let res = self.cmds.send((player, cmd));
        if res.is_err() {
            {
                let mut clients = self.clients.lock().unwrap();
                let _ = clients.remove(&player);
            };
            {
                let mut games = self.games.lock().unwrap();
                let _ = games.front_mut().unwrap().actors.remove(player);
            }
            info!("Player {} left the game -- disconnected when sending", player);
        };
        res
    }
}

impl Server {
    pub fn new(spec: GameSpec, game: Game) -> Server {
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

    pub fn handle(&self) -> ServerHandle {
        ServerHandle{
            spec: self.spec.clone(),
            games: self.games.clone(),
            clients: self.clients.clone(),
            cmds: self.cmds_tx.clone(),
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
                        let new_game = games.front().unwrap().advance(&self.spec, &inputs, TIME_STEP);
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
