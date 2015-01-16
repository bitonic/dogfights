#![feature(slicing_syntax)]
#![warn(unused_results)]
#![allow(unstable)]
extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;
#[macro_use] extern crate log;

extern crate bincode;
extern crate network;
extern crate geometry;
extern crate physics;
extern crate specs;
extern crate actors;
extern crate render;
extern crate conf;
extern crate input;

use rustc_serialize::{Encodable, Encoder, Decoder};
use sdl2::SdlResult;
use sdl2::pixels::Color;
use sdl2::render::Renderer;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::slice::SliceExt;
use std::sync::{Arc, Mutex};
use std::thread::Thread;
use std::sync::mpsc::{channel, Receiver};

use actors::*;
use conf::*;
use geometry::*;
use input::*;
use render::*;
use specs::*;

#[derive(PartialEq, Clone, Show, RustcEncodable, RustcDecodable)]
struct Game {
    actors: Actors,
    time: f32,
}

#[derive(PartialEq, Clone, Copy, Show)]
struct ShipInput {
    ship: ActorId,
    input: Input,
}

impl ShipInput {
    fn lookup(inputs: &Vec<ShipInput>, actor_id: ActorId) -> Option<Input> {
        for input in inputs.iter() {
            if input.ship == actor_id { return Some(input.input) }
        };
        None
    }
}

impl Game {
    fn advance(&self, spec: &GameSpec, inputs: &Vec<ShipInput>, dt: f32) -> Game {
        // First move everything, spawn new stuff
        let mut advanced_actors = Actors::prepare_new(&self.actors);
        for (actor_id, actor) in self.actors.iter() {
            let actor_input = ShipInput::lookup(inputs, *actor_id);
            match actor.advance(spec, &mut advanced_actors, actor_input, dt) {
                None                 => {},
                Some(advanced_actor) => { advanced_actors.insert(*actor_id, advanced_actor) },
            }
        };
        
        // Then compute interactions
        let mut interacted_actors = Actors::prepare_new(&advanced_actors);
        for (actor_id, actor) in advanced_actors.iter() {
            match actor.interact(spec, &advanced_actors) {
                None                   => {},
                Some(interacted_actor) => { interacted_actors.insert(*actor_id, interacted_actor) },
            }
        };

        // Done
        Game{
            actors: interacted_actors,
            time: self.time + dt,
        }
    }

    fn add_ship(&mut self, spec: &GameSpec) -> ActorId {
        let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
        self.actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)))
    }
}

fn render_game(render: &RenderEnv, game: &Game, spec: &GameSpec, trans: &Transform) -> SdlResult<()> {
    try!(render.map(&spec.map, &trans.pos));
    try!(render.actors(&game.actors, spec, trans));
    Ok(())
}

// ---------------------------------------------------------------------
// Server

type SnapshotId = u32;

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
                            render_game(render, &state.game, spec, &camera.transform()).ok().expect("Couldn't render game");
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
        render_game(render, &self.game, spec, &camera.transform())
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
                state = current;
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

// ---------------------------------------------------------------------
// Init

const PLANES_TEXTURE_ID: TextureId = 0;
const MAP_TEXTURE_ID: TextureId = 1;

fn init_sdl(vsync: bool) -> Renderer {
    sdl2::init(sdl2::INIT_EVERYTHING | sdl2::INIT_TIMER);
    let window = sdl2::video::Window::new(
        "Dogfights",
        sdl2::video::WindowPos::PosUndefined, sdl2::video::WindowPos::PosUndefined,
        (SCREEN_WIDTH as isize), (SCREEN_HEIGHT as isize),
        sdl2::video::SHOWN).ok().unwrap();
    let vsync_flag = if vsync { sdl2::render::PRESENTVSYNC } else { sdl2::render::RendererFlags::empty() };
    let flags = sdl2::render::ACCELERATED | vsync_flag;
    let renderer = Renderer::from_window(window, sdl2::render::RenderDriverIndex::Auto, flags).ok().unwrap();
    renderer.set_logical_size((SCREEN_WIDTH as isize), (SCREEN_HEIGHT as isize)).ok().unwrap();
    renderer
}

fn init_headless_sdl() {
    sdl2::init(sdl2::INIT_TIMER);
}

fn init_textures(renderer: &Renderer) -> Textures {
    let mut textures = HashMap::new();

    let planes_surface: sdl2::surface::Surface =
        sdl2_image::LoadSurface::from_file(&("assets/planes.png".parse()).unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture = renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let _ = textures.insert(PLANES_TEXTURE_ID, planes_texture);

    let map_surface = sdl2_image::LoadSurface::from_file(&("assets/background.png".parse()).unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let _ = textures.insert(MAP_TEXTURE_ID, map_texture);

    textures
}

fn init_spec() -> GameSpec {
    // Specs
    let mut specs = Vec::new();
    let bullet_spec = BulletSpec {
        sprite: Sprite {
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 424., y: 140.}, w: 3., h: 12.},
            center: Vec2{x: 1., y: 6.},
            angle: 90.,
        },
        vel: 1000.,
        lifetime: 5000.,
        bbox: BBox {
            rects: vec![
                Rect{
                    pos: Vec2{y: -1.5, x: -6.},
                    h: 3.,
                    w: 12.
                }]
        },
    };
    let bullet_spec_id = 0;
    specs.push(Spec::BulletSpec(bullet_spec));
    let ship_spec = ShipSpec {
        rotation_vel: 10.,
        rotation_vel_accel: 1.,
        accel: 800.,
        friction: 0.02,
        gravity: 100.,
        sprite: Sprite{
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 128., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        sprite_accel: Sprite {
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 88., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        bullet_spec: bullet_spec_id,
        firing_interval: 1.,
        shoot_from: Vec2{x: 18., y: 0.},
        bbox: BBox{
            rects: vec![
                Rect{
                    pos: Vec2{x: -12., y: -5.5},
                    w: 25.,
                    h: 11.
                },
                Rect{
                    pos: Vec2{x: 0., y: -15.},
                    w: 7.5,
                    h: 30.
                }
                ]
        },
    };
    let ship_spec_id: SpecId = 1;
    specs.push(Spec::ShipSpec(ship_spec));
    let shooter_spec = ShooterSpec {
        sprite: Sprite {
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 48., y: 248.}, w: 32., h: 24.},
            center: Vec2{x: 16., y: 12.},
            angle: 90.,
        },
        trans: Transform {
            pos: Vec2{x: 1000., y: 200.},
            rotation: to_radians(270.),
        },
        bullet_spec: bullet_spec_id,
        firing_rate: 2.,
    };
    let shooter_spec_id: SpecId = 2;
    specs.push(Spec::ShooterSpec(shooter_spec));
    let map = Map {
        w: SCREEN_WIDTH*10.,
        h: SCREEN_HEIGHT*10.,
        background_color: Color::RGB(0x58, 0xB7, 0xFF),
        background_texture: MAP_TEXTURE_ID,
    };
    let camera_spec = CameraSpec {
        accel: 1.2,
        h_pad: 220.,
        v_pad: 220. * SCREEN_HEIGHT / SCREEN_WIDTH,
    };
    GameSpec{
        map: map,
        camera_spec: camera_spec,
        ship_spec: ship_spec_id,
        shooter_spec: shooter_spec_id,
        specs: specs,
    }
}

// ---------------------------------------------------------------------
// tests

#[test]
fn test_encoding() {
    let ship = Ship{
        spec: 1,
        trans: Transform::pos(Vec2 { x: 400., y: 300.005 }),
        vel: Vec2 { x: 0., y: 0.9999 },
        not_firing_for: 100000.01,
        accel: false,
        rotating: Rotating::Still,
        camera: Camera { pos: Vec2 { x: 0., y: 0.011999 }, vel: Vec2 { x: 0., y: 1.19988 } }
    };
    let mut actors = Actors::new();
    actors.insert(0, Actor::Ship(ship));
    let game = PlayerGame{
        game: Game { actors: actors },
        player_id: 0
    };
    assert!(game == bincode::decode(bincode::encode(&game).ok().unwrap()).ok().unwrap());
}
