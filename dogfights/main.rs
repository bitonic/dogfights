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
extern crate ai;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::slice::SliceExt;
use std::thread::Thread;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::io::{IoErrorKind};

use actors::*;
use input::*;
use render::*;
use init::*;
use server::*;

mod init;

pub fn run_local(ais: Vec<String>) {
    let renderer = init_sdl(false);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = Arc::new(init_spec());
    let server = Server::new(spec.clone(), Game::new());
    let (player, mut client_send, mut client_recv) = server.join_handle().join();

    // Add ais
    for ai_s in ais.iter() {
        let ai = ai::parse_ai_string(&**ai_s, Some(player));
        let (_, mut ai_send, mut ai_recv) = server.join_handle().join();
        let _ = Thread::spawn(move || { attach_ai(&mut ai_send, &mut ai_recv, ai.deref(), |_| {}) });
    }

    // Thread running the server
    let _ = Thread::spawn(move || { server.run(); });

    attach_sdl(&mut client_send, &mut client_recv, |game| {
        render.player_game(&game, spec.deref()).ok().unwrap();
        render.renderer.present();
    });
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
    let spec = Arc::new(init_spec());
    let server = Server::new(spec.clone(), Game::new());
    let join_handle = server.join_handle();

    let clients: Arc<Mutex<HashMap<SocketAddr, ServerClientSend>>> = Arc::new(Mutex::new(HashMap::new()));
    let worker_clients = clients.clone();

    // Thread running the server
    let _ = Thread::spawn(move || { server.run(); });

    loop {
        if should_quit() { break };

        let (addr, input): (SocketAddr, Input) = net.recv().ok().unwrap();
        match clients.lock().unwrap().entry(addr) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().send_input(input);
            },
            Entry::Vacant(entry) => {
                let (player, mut player_send, mut player_recv) = join_handle.join();
                info!("New player {} for connection {}", player, addr);
                let _ = entry.insert(player_send.clone());
                let mut worker_net = net.clone();
                let clients = worker_clients.clone();
                let _ = Thread::spawn(move || {
                    loop {
                        // It it was error, it'd mean that the server
                        // has removed the player, for some reason
                        let mb_game = player_recv.recv_game();
                        match mb_game {
                            None => break,
                            Some(game) => {
                                let send_res = worker_net.send(addr, &game);
                                match send_res {
                                    Ok(()) => (),
                                    Err(err) => match err.kind {
                                        IoErrorKind::Closed => {
                                            let _ = clients.lock().unwrap().remove(&addr);
                                            break
                                        },
                                        _ => (), // Just ignore it
                                    }
                                };
                            }
                        }
                    }
                });
                let _ = player_send.send_input(input);
            }
        };
    }
}

pub fn run_remote<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind: B) {
    let client = network::Client::new(server_addr, bind, true).ok().unwrap();
    let mut client_handle_send = client.handle();
    let mut client_handle_recv = client.handle();

    let renderer = init_sdl(false);
    let textures = init_textures(&renderer);
    let render = RenderEnv{renderer: renderer, textures: textures};
    let spec = Arc::new(init_spec());

    attach_sdl(&mut client_handle_send, &mut client_handle_recv, |game| {
        render.player_game(&game, spec.deref()).ok().unwrap();
        render.renderer.present();
    });
}

pub fn run_remote_ai<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind: B, ai_s: &str, display: bool) {
    let client = network::Client::new(server_addr, bind, true).ok().unwrap();
    let mut client_handle_send = client.handle();
    let mut client_handle_recv = client.handle();

    let ai = ai::parse_ai_string(ai_s, None);

    let mb_render: Option<RenderEnv> = if display {
        let renderer = init_sdl(false);
        let textures = init_textures(&renderer);
        let render = RenderEnv{renderer: renderer, textures: textures};
        Some(render)
    } else {
        init_headless_sdl();
        None
    };

    let spec = init_spec();

    attach_ai(&mut client_handle_send, &mut client_handle_recv, ai.deref(), |player_game| {
        match mb_render {
            None => (),
            Some(ref render) => {
                render.player_game(&player_game, &spec).ok().unwrap();
                render.renderer.present();
            }
        }
    });
}
