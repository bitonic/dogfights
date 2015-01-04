#![feature(associated_types)]
#![feature(default_type_params)]
#![feature(globs)]
#![feature(old_orphan_check)]
#![feature(slicing_syntax)]
extern crate bincode;
extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;

use rustc_serialize::{Encodable, Encoder, Decoder};
use sdl2::SdlResult;
use sdl2::pixels::Color;
use sdl2::render::Renderer;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::io::{IoResult};
use std::slice::SliceExt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{Thread, JoinGuard};
use std::ops::Deref;

use actors::*;
use constants::*;
use geometry::*;
use input::*;
use render::*;
use specs::*;

pub mod actors;
pub mod constants;
pub mod geometry;
pub mod input;
pub mod network;
pub mod physics;
pub mod render;
pub mod specs;

#[derive(PartialEq, Clone, Show, RustcEncodable, RustcDecodable)]
struct Game {
    actors: Actors,
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
        for (actor_id, actor) in self.actors.actors.iter() {
            let actor_input = ShipInput::lookup(inputs, *actor_id);
            match actor.advance(spec, &mut advanced_actors, actor_input, dt) {
                None                 => {},
                Some(advanced_actor) => { advanced_actors.insert(*actor_id, advanced_actor) },
            }
        };
        
        // Then compute interactions
        let mut interacted_actors = Actors::prepare_new(&advanced_actors);
        for (actor_id, actor) in advanced_actors.actors.iter() {
            match actor.interact(spec, &advanced_actors) {
                None                   => {},
                Some(interacted_actor) => { interacted_actors.insert(*actor_id, interacted_actor) },
            }
        };

        // Done
        Game{
            actors: interacted_actors,
        }
    }

    fn add_ship(&mut self, spec: &GameSpec) -> ActorId {
        let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
        self.actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)))
    }
}

fn render_game(game: &Game, spec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    try!(render_map(spec.map, renderer, &trans.pos));
    try!(render_actors(&game.actors, spec, renderer, trans));
    Ok(())
}

// ---------------------------------------------------------------------
// Server

type SnapshotId = u32;

#[derive(PartialEq, Clone)]
struct Server<'a> {
    game_spec: &'a GameSpec<'a>,
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

    fn new(spec: &'a GameSpec<'a>, game: Game) -> Server<'a> {
        Server{
            game_spec: spec,
            game: game,
            clients: HashMap::new(),
        }
    }

    fn worker(queue: Arc<Mutex<Vec<Message>>>, server: &mut network::Server) -> ! {
        loop {
            let (addr, input): (SocketAddr, IoResult<Input>) =
                server.recv().ok().expect("Server.worker: Could not receive");
            match input {
                Err(err) =>
                    println!("Server.worker: couldn't decode message: {}", err),
                Ok(input) => {
                    let msg = Message{from: addr, input: input};
                    {
                        let mut msgs = queue.lock().unwrap();
                        msgs.push(msg);
                    }
                }
            }
        }
    }

    fn drain(queue: Arc<Mutex<Vec<Message>>>) -> Vec<Message> {
        let mut msgs: MutexGuard<Vec<Message>> = queue.lock().unwrap();
        let old_msgs: Vec<Message> = msgs.deref().clone();
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
                    let _ = entry.set(ship_id);
                }
            }
        };
        inputs
    }

    fn broadcast_game(&self, server: &mut network::Server) -> IoResult<()> {
        // FIXME: encode once
        for (addr, player_id) in self.clients.iter() {
            // FIXME: encode more efficiently...
            let remote_game = PlayerGame{
                game: self.game.clone(),
                player_id: *player_id,
            };
            try!(server.send(*addr, &remote_game));
        };
        Ok(())
    }

    fn run<A: ToSocketAddr>(self, addr: &A) {
        let addr = addr.to_socket_addr().ok().expect("Server.run: could not get SocketAddr");
        let mut server = network::Server::new(addr).ok().expect("Server.worker: Could not create network server");
        let mut worker_server = server.clone();
        let queue_local = Arc::new(Mutex::new(Vec::new()));
        let queue_worker = queue_local.clone();
        let guard: JoinGuard<()> = Thread::spawn(move || {
            Server::worker(queue_worker, &mut worker_server)
        });
        guard.detach();

        let wait_ms = (TIME_STEP * 1000.) as uint;
        let mut state = self;
        loop {
            if Server::should_quit() { break }

            let inputs = Server::drain(queue_local.clone());
            let inputs = state.prepare_inputs(&inputs);
            state.game = state.game.advance(state.game_spec, &inputs, TIME_STEP);
            state.broadcast_game(&mut server.clone()).ok().expect("Couldn't broadcast messages");

            // FIXME: maybe time more precisely -- e.g. take into
            // account the time it took to generate the state
            sdl2::timer::delay(wait_ms);
        }
    }
}

pub fn server<A: ToSocketAddr>(addr: &A) {
    let renderer = init_sdl();
    game_spec(&renderer, |spec| {
        // Actors
        let game = Game{actors: Actors::new()};
        let server = Server::new(spec, game);
        server.run(addr);
    })
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

    fn render(&self, spec: &GameSpec, renderer: &Renderer) -> SdlResult<()> {
        let camera = self.game.actors.get(self.player_id).is_ship().camera;
        render_game(&self.game, spec, renderer, &camera.transform())
    }

    fn run(self, spec: &GameSpec, renderer: &Renderer) {
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
            input.process_events();
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

            state.render(spec, renderer).ok().expect("Failed to render the state");
            renderer.present();
        }
    }
}

pub fn client() {
    let renderer = init_sdl();
    game_spec(&renderer, |spec| {
        // Actors
        let mut actors = Actors::new();
        let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
        let player_id = actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)));
        let _ = actors.add(Actor::Shooter(
            Shooter{
                spec: spec.shooter_spec,
                time_since_fire: 0.,
            }));
        let game = Game{actors: actors};

        let client = PlayerGame{
            game: game,
            player_id: player_id,
        };
        client.run(spec, &renderer);
    })
}

pub fn remote_client<A: ToSocketAddr, B: ToSocketAddr>(server_addr: A, bind_addr: B) {
    let renderer = init_sdl();
    let mut client =
        network::Client::new(server_addr, bind_addr).ok().expect("remote_client: could not create network client");
    game_spec(&renderer, |spec| {
        let mut input = Input{
            quit: false,
            accel: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        };
        loop {
            input.process_events();
            if input.quit { break }

            client.send(&input).ok().expect("remote_client: couldn't send to server");
            loop {
                let game: IoResult<PlayerGame> =
                    client.recv().ok().expect("remote_client: couldn't receive from server");
                match game {
                    Ok(game) => {
                        game.render(spec, &renderer).ok().expect("remote_client: couldn't render");
                        renderer.present();
                        break
                    },
                    Err(err) => {
                        println!("Error while receiving state from server: {}", err)
                    },
                }
            };
        }});
}

// ---------------------------------------------------------------------
// Init

fn init_sdl() -> Renderer {
    sdl2::init(sdl2::INIT_VIDEO);
    let window = sdl2::video::Window::new(
        "Dogfights",
        sdl2::video::WindowPos::PosUndefined, sdl2::video::WindowPos::PosUndefined,
        (SCREEN_WIDTH as int), (SCREEN_HEIGHT as int),
        sdl2::video::SHOWN).ok().unwrap();
    let renderer = Renderer::from_window(
        window,
        sdl2::render::RenderDriverIndex::Auto,
        sdl2::render::ACCELERATED | sdl2::render::PRESENTVSYNC).ok().unwrap();
    renderer.set_logical_size((SCREEN_WIDTH as int), (SCREEN_HEIGHT as int)).ok().unwrap();
    renderer
}

fn game_spec<T, F: FnOnce(&GameSpec) -> T>(renderer: &Renderer, cont: F) -> T {
    // Specs
    let planes_surface: sdl2::surface::Surface =
        sdl2_image::LoadSurface::from_file(&("assets/planes.png".parse()).unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture = renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let mut specs = Vec::new();
    let bullet_spec = BulletSpec {
        sprite: &Sprite {
            texture: &planes_texture,
            rect: Rect{pos: Vec2{x: 424., y: 140.}, w: 3., h: 12.},
            center: Vec2{x: 1., y: 6.},
            angle: 90.,
        },
        vel: 1000.,
        lifetime: 5000.,
        bbox: &BBox {
            rects: &[
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
        sprite: &Sprite{
            texture: &planes_texture,
            rect: Rect{pos: Vec2{x: 128., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        sprite_accel: &Sprite {
            texture: &planes_texture,
            rect: Rect{pos: Vec2{x: 88., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        bullet_spec: bullet_spec_id,
        firing_interval: 1.,
        shoot_from: Vec2{x: 18., y: 0.},
        bbox: &BBox{
            rects: &[
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
        sprite: &Sprite {
            texture: &planes_texture,
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
    let map_surface = sdl2_image::LoadSurface::from_file(&("assets/background.png".parse()).unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let map = &Map {
        w: SCREEN_WIDTH*10.,
        h: SCREEN_HEIGHT*10.,
        background_color: Color::RGB(0x58, 0xB7, 0xFF),
        background_texture: &map_texture,
    };
    let camera_spec = &CameraSpec {
        accel: 1.2,
        h_pad: 220.,
        v_pad: 220. * SCREEN_HEIGHT / SCREEN_WIDTH,
    };
    cont(&GameSpec{
        map: map,
        camera_spec: camera_spec,
        ship_spec: ship_spec_id,
        shooter_spec: shooter_spec_id,
        specs: specs.as_slice(),
    })
}
