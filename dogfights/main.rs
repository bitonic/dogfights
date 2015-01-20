#![feature(slicing_syntax)]
#![warn(unused_results)]
#![allow(unstable)]
extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;
#[macro_use] extern crate log;

extern crate actors;
extern crate input;
extern crate render;
extern crate server;
extern crate geometry;
extern crate conf;
extern crate specs;
extern crate network;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::slice::SliceExt;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread::Thread;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use actors::*;
use input::*;
use render::*;
use init::*;
use server::*;

mod init;


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

    let clients: Arc<Mutex<HashMap<SocketAddr, ActorId>>> = Arc::new(Mutex::new(HashMap::new()));
    let worker_clients = clients.clone();

    // Thread running the server
    let _ = Thread::spawn(move || { server.run(); });

    loop {
        if should_quit() { break };

        let (addr, input): (SocketAddr, Input) = net.recv().ok().unwrap();
        let player = {
            match clients.lock().unwrap().entry(addr) {
                Entry::Occupied(entry) => {
                    *entry.get()
                },
                Entry::Vacant(entry) => {
                    let (player, rx) = server_handle.join();
                    info!("New player {} for connection {}", player, addr);
                    let _ = entry.insert(player);
                    let mut worker_net = net.clone();
                    let clients = worker_clients.clone();
                    let _ = Thread::spawn(move || {
                        loop {
                            // It it was error, it'd mean that the server
                            // has removed the player, for some reason
                            let game = rx.recv().ok().unwrap();
                            let send_res = worker_net.send(addr, &PlayerGame{player: player, game: game});
                            if network::is_disconnect(&send_res) {
                                let _ = clients.lock().unwrap().remove(&addr);
                                break
                            };
                            send_res.ok().unwrap();
                        }
                    });
                    player
                }
            }
        };
        // This would mean that the server has died
        server_handle.send(player, input).ok().unwrap();
    }
}

pub fn run_remote<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind: B) {
    let client = network::Client::new(server_addr, bind, true).ok().unwrap();
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
        let mut first = true;

        loop {
            let new_input = input.process_events();
            if new_input.quit {
                let _ = quit_tx.send(());
                break
            }
            if first || new_input != input {
                first = false;
                input = new_input;
                let send_res = client_sender.send(&input);
                if network::is_disconnect(&send_res) { break; };
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
        let recv_res = client_receiver.recv();
        if network::is_disconnect(&recv_res) { break; }
        if !network::is_timeout(&recv_res) {
            let game = recv_res.ok().unwrap();
            render.player_game(&game, &spec).ok().unwrap();
            render.renderer.present();
        }
    };
}

