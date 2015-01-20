#![feature(slicing_syntax)]
#![warn(unused_results)]
#![allow(unstable)]
extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;
#[macro_use] extern crate log;

extern crate actors;
extern crate bincode;
extern crate conf;
extern crate geometry;
extern crate input;
extern crate interpolate;
extern crate network;
extern crate physics;
extern crate render;
extern crate specs;
extern crate server;

use rustc_serialize::{Encodable, Encoder, Decoder};
use sdl2::SdlResult;
use sdl2::pixels::Color;
use sdl2::render::Renderer;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::slice::SliceExt;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::Thread;
use std::ops::Deref;

use actors::*;
use conf::*;
use geometry::*;
use input::*;
use interpolate::*;
use render::*;
use specs::*;
use init::*;
use server::*;

mod init;

/*
// ---------------------------------------------------------------------
// Server

#[derive(PartialEq, Clone)]
struct Server<'a> {
    game_spec: &'a GameSpec,
    game: Game,
    clients: HashMap<SocketAddr, ActorId>,
}

#[derive(PartialEq, Clone)]
struct Message {
    from: SocketAddr,
    input: Input,
}

impl<'a> Server<'a> {
    fn should_quit() -> bool {
        loop {
            match sdl2::event::poll_event() {
                sdl2::event::Event::None    => break,
                sdl2::event::Event::Quit(_) => return true,
                _                           => {},
            }
        };
        false
    }

    fn new(spec: &'a GameSpec, game: Game) -> Server<'a> {
        Server{
            game_spec: spec,
            game: game,
            clients: HashMap::new(),
        }
    }

    fn worker(queue: Arc<Mutex<Vec<Message>>>, quit: Receiver<()>, server: &mut network::Server) {
        loop {
            let quit = quit.try_recv().is_ok();
            if quit {
                break;
            };

            let result: Option<(SocketAddr, Input)> = network::handle_recv_result(server.recv());
            match result {
                None => (),
                Some((addr, input)) => {
                    let msg = Message{from: addr, input: input};
                    let mut msgs = queue.lock().unwrap();
                    msgs.push(msg);
                }
            }
        }
    }

    fn drain(queue: Arc<Mutex<Vec<Message>>>) -> Vec<Message> {
        let mut msgs = queue.lock().unwrap();
        let old_msgs = msgs.clone();
        msgs.clear();
        old_msgs
    }

    fn prepare_inputs(&mut self, msgs: &Vec<Message>) -> Vec<ShipInput> {
        let mut inputs = Vec::new();
        for msg in msgs.iter() {
            match self.clients.entry(msg.from) {
                Entry::Occupied(entry) =>
                    inputs.push(ShipInput{ship: *entry.get(), input: msg.input}),
                Entry::Vacant(entry) => {
                    let ship_id = self.game.add_ship(self.game_spec);
                    let _ = entry.insert(ship_id);
                }
            }
        };
        inputs
    }

    fn broadcast_game(&mut self, server: &mut network::Server) {
        // FIXME: encode once
        let mut dead: Vec<(SocketAddr, ActorId)> = Vec::new();

        for (addr, player_id) in self.clients.iter() {
            // FIXME: encode more efficiently...
            let remote_game = PlayerGame{
                game: self.game.clone(),
                player_id: *player_id,
            };
            match server.send(*addr, &remote_game) {
                Ok(()) => {},
                Err(err) => debug!("Server::broadcast_game: got error {}", err),
            };
            // Remove if inactive
            if !server.active_conn(addr) {
                dead.push((*addr, *player_id));
            }
        };

        for client in dead.iter() {
            let _ = self.clients.remove(&client.0).unwrap();
            self.game.actors.remove(client.1);
        }
    }

    fn run<A: ToSocketAddr>(self, addr: &A, render: &Option<RenderEnv>, spec: &GameSpec) {
        let addr = addr.to_socket_addr().ok().expect("Server.run: could not get SocketAddr");
        let server = network::Server::new(addr).ok().expect("Server.worker: Could not create network server");
        let mut worker_server = server.clone();
        let queue_local = Arc::new(Mutex::new(Vec::new()));
        let queue_worker = queue_local.clone();
        let (quit_tx, quit_rx) = channel();
        let _ = Thread::spawn(move || {
            Server::worker(queue_worker, quit_rx, &mut worker_server)
        });

        let wait_ms = (TIME_STEP * 1000.) as usize;
        let mut state = self;
        let mut player_id: Option<ActorId> = None;
        let mut prev_time = sdl2::get_ticks();
        loop {
            let time = sdl2::get_ticks();
            debug!("Server::run: main loop.  Time interval: {}", time - prev_time);
            prev_time = time;

            if Server::should_quit() { break }

            let inputs = Server::drain(queue_local.clone());
            let inputs = state.prepare_inputs(&inputs);
            state.game = state.game.advance(state.game_spec, &inputs, TIME_STEP);
            state.broadcast_game(&mut server.clone());

            // Render if we have at least one player id to follow
            match *render {
                None => (),
                Some(ref render) => {
                    if state.game.actors.len() == 0 {
                        player_id = None;
                    };
                    if player_id.is_none() {
                        for first_player_id in state.game.actors.keys() {
                            player_id = Some(*first_player_id);
                            break
                        }
                    };
                    match player_id {
                        Some(player_id) => {
                            let camera = state.game.actors.get(player_id).unwrap().is_ship().camera;
                            render.game(&state.game, spec, &camera.transform()).ok().expect("Couldn't render game");
                            render.renderer.present();
                        },
                        None => {}
                    }
                },
            };

            // FIXME: maybe time more precisely -- e.g. take into
            // account the time it took to generate the state
            sdl2::timer::delay(wait_ms);
        }

        quit_tx.send(()).ok().unwrap();
    }
}

pub fn server<A: ToSocketAddr>(addr: &A, display: bool) {
    let render = if display {
        let renderer = init_sdl(false);
        let textures = init_textures(&renderer);
        Some(RenderEnv{
            renderer: renderer,
            textures: textures,
        })
    } else {
        init_headless_sdl();
        None
    };
    let spec = init_spec();
    let game = Game{actors: Actors::new(), time: 0.};
    let server = Server::new(&spec, game);
    server.run(addr, &render, &spec);
}

// ---------------------------------------------------------------------
// PlayerGame

#[derive(PartialEq, Clone, Show, RustcEncodable, RustcDecodable)]
struct PlayerGame {
    game: Game,
    player_id: ActorId,
}

impl PlayerGame {
    fn advance(&self, spec: &GameSpec, input: &Input, dt: f32) -> PlayerGame {
        let inputs = vec![ShipInput{ship: self.player_id, input: *input}];
        let game = self.game.advance(spec, &inputs, dt);
        PlayerGame{
            game: game,
            player_id: self.player_id
        }
    }

    fn render(&self, render: &RenderEnv, spec: &GameSpec) -> SdlResult<()> {
        let camera = self.game.actors.get(self.player_id).unwrap().is_ship().camera;
        render.game(&self.game, spec, &camera.transform())
    }

    fn run(self, render: &RenderEnv, spec: &GameSpec) {
        let mut prev_time = sdl2::get_ticks();
        let mut accumulator = 0.;
        let mut state = self;
        let mut input = Input{
            quit: false,
            accel: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        };
        loop {
            input = input.process_events();
            if input.quit { break };

            let time_now = sdl2::get_ticks();
            let frame_time = ((time_now - prev_time) as f32) / 1000.; // Seconds to millis
            let frame_time = if frame_time > MAX_FRAME_TIME { MAX_FRAME_TIME } else { frame_time };
            prev_time = time_now;
            accumulator += frame_time;

            let mut previous = state.clone();
            if accumulator >= TIME_STEP {
                accumulator -= TIME_STEP;
                let mut current = previous.advance(spec, &input, TIME_STEP);
                while accumulator >= TIME_STEP {
                    accumulator -= TIME_STEP;
                    let new = current.advance(spec, &input, TIME_STEP);
                    previous = current;
                    current = new;
                }
                // TODO: interpolate previous and current
                state = PlayerGame{
                    player_id: current.player_id,
                    game: interpolate_game(&previous.game, &current.game, accumulator / TIME_STEP),
                };
            }

            state.render(render, spec).ok().expect("Failed to render the state");
            render.renderer.present();
        }
    }
}

pub fn client() {
    let renderer = init_sdl(true);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = init_spec();
    // Actors
    let mut actors = Actors::new();
    let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
    let player_id = actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)));
    let _ = actors.add(Actor::Shooter(
        Shooter{
            spec: spec.shooter_spec,
            time_since_fire: 0.,
        }));
    let game = Game{actors: actors, time: 0.};
    
    let client = PlayerGame{
        game: game,
        player_id: player_id,
    };
    client.run(&render, &spec);
}

pub fn remote_client<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind_addr: B) {
    let renderer = init_sdl(false);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = init_spec();
    let mut client = match network::Client::new(server_addr, bind_addr) {
        Err(err) => panic!("remote_client: could not create network client: {}", err),
        Ok(x)    => x
    };
    let mut worker_handle = client.handle().clone();
    let (tx, rx) = channel();
    let _ = Thread::spawn(move || {
        loop {
            let res: Option<PlayerGame> = network::handle_recv_result(worker_handle.recv());
            match res {
                None => (),
                Some(game) => tx.send(game).ok().unwrap(),
            }
        }
    });

    let mut input = Input{
        quit: false,
        accel: false,
        firing: false,
        rotating: Rotating::Still,
        paused: false,
    };
    let mut prev_input;
    // Send the first to make sure the server knows we're there.
    match client.handle().send(&input) {
        Err(err) => panic!("remote_client: couldn't send to server {}", err),
        Ok(()) => (),
    };
    loop {
        prev_input = input;
        input = input.process_events();
        debug!("Input: {:?}", input);
        if input.quit {
            debug!("Quitting");
            break
        }
        if input != prev_input {
            debug!("Input changed, sending it");
            client.handle().send(&input).ok().expect("remote_client: couldn't send to server");
        }
        let game: PlayerGame = rx.recv().ok().unwrap();
        game.render(&render, &spec).ok().expect("remote_client: couldn't render");
        render.renderer.present();
    }
}
*/

pub fn run_local() {
    let renderer = init_sdl(false);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = init_spec();
    let server = Server::new(spec.clone(), Game::new());
    let server_handle = server.handle();
    let (player, rx) = server_handle.join();

    // Thread running the server
    let _ = Thread::spawn(move || { server.run(); });

    let (quit_tx, quit_rx) = channel();

    // Thread sending inputs
    let _ = Thread::spawn(move || {
        // Send input every 10ms
        let mut input = Input{
            quit: false,
            accel: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        };

        loop {
            let new_input = input.process_events();
            if new_input.quit {
                let _ = quit_tx.send(());
                break
            }
            if new_input != input {
                input = new_input;
                let mb_err = server_handle.send(player, input);
                if mb_err.is_err() { break };
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

        match rx.recv() {
            Err(_) => break,
            Ok(game) => {
                render.game(game.deref(), &spec, player).ok().unwrap();
                render.renderer.present();
            }
        }
    };
}

fn should_quit() -> bool {
    loop {
        match sdl2::event::poll_event() {
            sdl2::event::Event::None    => break,
            sdl2::event::Event::Quit(_) => return true,
            _                           => (),
        }
    };
    false
}

pub fn run_server<A: ToSocketAddr>(addr: A) {
    let mut net = network::Server::new(addr).ok().unwrap();
    init_headless_sdl();
    let spec = init_spec();
    let server = Server::new(spec.clone(), Game::new());
    let server_handle = server.handle();

    let mut clients: HashMap<SocketAddr, ActorId> = HashMap::new();

    // Thread running the server
    let _ = Thread::spawn(move || { server.run(); });

    loop {
        if should_quit() { break };

        match network::handle_recv_result(net.recv()) {
            None => (),
            Some((addr, input)) => {
                let player = match clients.entry(addr) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        let (player, rx) = server_handle.join();
                        let _ = entry.insert(player);
                        let mut worker_net = net.clone();
                        let _ = Thread::spawn(move || {
                            loop {
                                match rx.recv() {
                                    Ok(game) => {
                                        // TODO what to do with this error?
                                        worker_net.send(addr, &PlayerGame{player: player, game: game});
                                        if !worker_net.active_conn(&addr) { break };
                                    }
                                    Err(_) => break
                                }
                              }
                        });
                        player
                    }
                };
                if server_handle.send(player, input).is_err() {
                    let _ = clients.remove(&addr);
                };
            }
        }
    }
}

pub fn run_remote<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind: B) {
    let mut client = network::Client::new(server_addr, bind, true).ok().unwrap();
    let mut client_sender = client.handle();
    let mut client_receiver = client.handle();

    let renderer = init_sdl(false);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = init_spec();

    let (quit_tx, quit_rx) = channel();

    // Thread sending inputs
    let _ = Thread::spawn(move || {
        // Send input every 10ms
        let mut input = Input{
            quit: false,
            accel: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        };

        loop {
            let new_input = input.process_events();
            if new_input.quit {
                let _ = quit_tx.send(());
                break
            }
            if new_input != input {
                input = new_input;
                // TODO what to do with this error?
                client_sender.send(&input);
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

        client_receiver.set_timeout(Some(5));
        match network::handle_recv_result(client_receiver.recv()) {
            None => (),
            Some(game) => {
                render.player_game(&game, &spec).ok().unwrap();
                render.renderer.present();
            }
        }
    };
}

