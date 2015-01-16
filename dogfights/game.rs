extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;
extern crate bincode;

use std::num::FloatMath;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{IoResult};
use std::io::net::ip::{SocketAddr, ToSocketAddr};
use std::comm::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{Thread, JoinGuard};
use sdl2::SdlResult;
use sdl2::pixels::Color;
use sdl2::render::{Renderer, Texture};
use rustc_serialize::{Encodable, Encoder};

use geometry::{to_radians, from_radians, Vec2, Rect, Transform};

pub mod geometry;
pub mod physics;
pub mod network;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: f32 = 800.;
static SCREEN_HEIGHT: f32 = 600.;

// 50 ms timesteps
const TIME_STEP: f32 = 0.01;
const MAX_FRAME_TIME: f32 = 0.250;

// ---------------------------------------------------------------------
// Bounding boxes

#[derive(PartialEq, Clone)]
struct BBox<'a> {
    rects: &'a [Rect],
}

impl<'a> BBox<'a> {
    fn overlapping(this: BBox, this_t: &Transform, other: BBox, other_t: &Transform) -> bool {
        let mut overlap = false;
        for this in this.rects.iter() {
            if overlap { break };
            for other in other.rects.iter() {
                if overlap { break };
                overlap = Rect::overlapping(this, this_t, other, other_t);
            }
        }
        overlap
    }

    fn render(&self, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        try!(renderer.set_draw_color(Color::RGB(0xFF, 0x00, 0x00)));
        for rect in self.rects.iter() {
            let (tl, tr, bl, br) = rect.transform(trans);
            try!(renderer.draw_line(tl.point(), tr.point()));
            try!(renderer.draw_line(tr.point(), br.point()));
            try!(renderer.draw_line(br.point(), bl.point()));
            try!(renderer.draw_line(bl.point(), tl.point()));
        };
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Sprites

#[derive(PartialEq, Clone, Copy)]
struct Sprite<'a> {
    texture: &'a Texture,
    rect: Rect,
    center: Vec2,
    // If the sprite is already rotated by some angle
    angle: f32,
}

impl<'a> Sprite<'a> {
    fn render(&self, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        let dst = Rect{
            pos: trans.pos - self.center,
            w: self.rect.w,
            h: self.rect.h
        };
        let angle = from_radians(trans.rotation);
        renderer.copy_ex(
            self.texture, Some(self.rect.sdl_rect()), Some(dst.sdl_rect()), self.angle - angle,
            Some(self.center.point()), sdl2::render::RendererFlip::None)
    }
}

// ---------------------------------------------------------------------
// Maps

#[derive(PartialEq, Clone, Copy)]
struct Map<'a> {
    w: f32,
    h: f32,
    background_color: Color, 
    background_texture: &'a Texture,
}

impl<'a> Map<'a> {
    fn bound(&self, p: Vec2) -> Vec2 {
        // TODO handle points that are badly negative
        fn f(n: f32, m: f32) -> f32 {
            if n < 0. {
                0.
            } else if n > m {
                m
            } else {
                n
            }
        };
        Vec2{x: f(p.x, self.w), y: f(p.y, self.h)}
    }

    fn bound_rect(&self, p: Vec2, w: f32, h: f32) -> Vec2 {
        fn f(n: f32, edge: f32, m: f32) -> f32 {
            if n < 0. {
                0.
            } else if n + edge > m {
                m - edge
            } else {
                n
            }
        };
        Vec2{x: f(p.x, w, self.w), y: f(p.y, h, self.h)}
    }

    fn render(&self, renderer: &Renderer, pos: &Vec2) -> SdlResult<()> {
        // Fill the whole screen with the background color
        try!(renderer.set_draw_color(self.background_color));
        let rect = sdl2::rect::Rect {
            x: 0, y: 0, w: SCREEN_WIDTH as i32, h: SCREEN_HEIGHT as i32
        };
        try!(renderer.fill_rect(&rect));

        // Fill with the background texture.  The assumption is that 4
        // background images are needed to cover the entire screen:
        // 
        // map
        // ┌──────────────────────────────────────────┐
        // │                  ┊                   ┊   │
        // │  pos             ┊                   ┊   │
        // │  ┌─────────────────────┐             ┊   │
        // │  │               ┊     │             ┊   │
        // │  │             t ┊     │             ┊   │
        // │┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│
        // │  │               ┊     │             ┊   │
        // │  └─────────────────────┘             ┊   │
        // │                  ┊                   ┊   │
        // └──────────────────────────────────────────┘

        let bgr = try!(self.background_texture.query());
        let bgr_w = bgr.width as f32;
        let bgr_h = bgr.height as f32;
        let t = Vec2 {
            x: bgr_w - (pos.x % bgr_w),
            y: bgr_h - (pos.y % bgr_h),
        };
        let top_left = Vec2 {
            x: t.x - bgr_w,
            y: t.y - bgr_h,
        };
        let top_right = Vec2 {
            x: t.x,
            y: t.y - bgr_h,
        };
        let bottom_left = Vec2 {
            x: t.x - bgr_w,
            y: t.y,
        };
        let bottom_right = Vec2 {
            x: t.x,
            y: t.y,
        };
        let to_rect = |p: Vec2| -> Option<sdl2::rect::Rect> {
            Some(sdl2::rect::Rect {
                x: p.x as i32,
                y: p.y as i32,
                w: bgr.width as i32,
                h: bgr.height as i32,
            })
        };
        
        try!(renderer.copy(self.background_texture, None, to_rect(top_left)));
        try!(renderer.copy(self.background_texture, None, to_rect(top_right)));
        try!(renderer.copy(self.background_texture, None, to_rect(bottom_left)));
        renderer.copy(self.background_texture, None, to_rect(bottom_right))
    }
}


// ---------------------------------------------------------------------
// Actors

type ActorId = usize;

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
enum Actor {
    Ship(Ship),
    Shooter(Shooter),
    Bullet(Bullet),
}

impl Actor {
    // Returns whether the actor is still alive
    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, input: Option<Input>, dt: f32) -> Option<Actor> {
        match *self {
            Actor::Ship(ref ship) =>
                ship.advance(sspec, actors, input, dt).map(|x| Actor::Ship(x)),
            Actor::Shooter(ref shooter) => {
                assert!(input.is_none());
                shooter.advance(sspec, actors, dt).map(|x| Actor::Shooter(x))
            },
            Actor::Bullet(ref bullet) => {
                assert!(input.is_none());
                bullet.advance(sspec, actors, dt).map(|x| Actor::Bullet(x))
            },
        }
    }

    fn interact<'a>(&self, _: &GameSpec<'a>, _: &Actors) -> Option<Actor> {
        Some(*self)
    }

    fn render<'a>(&self, sspec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        match *self {
            Actor::Ship(ref ship) => ship.render(sspec, renderer, trans),
            Actor::Shooter(ref shooter) => shooter.render(sspec, renderer, trans),
            Actor::Bullet(ref bullet) => bullet.render(sspec, renderer, trans),
        }
    }

    fn is_ship(&self) -> &Ship {
        match *self {
            Actor::Ship(ref ship) => ship,
            _                     => unreachable!(),
        }
    }
}

type SpecId = usize;

#[derive(PartialEq, Clone, Copy)]
enum Spec<'a> {
    ShipSpec(ShipSpec<'a>),
    ShooterSpec(ShooterSpec<'a>),
    BulletSpec(BulletSpec<'a>),
}

impl<'a> Spec<'a> {
    fn is_ship(&self) -> &ShipSpec<'a> {
        match *self {
            Spec::ShipSpec(ref spec) => spec,
            _                        => unreachable!(),
        }
    }

    fn is_shooter(&self) -> &ShooterSpec<'a> {
        match *self {
            Spec::ShooterSpec(ref spec) => spec,
            _                           => unreachable!(),
        }
    }

    fn is_bullet(&self) -> &BulletSpec<'a> {
        match *self {
            Spec::BulletSpec(ref spec) => spec,
            _                          => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------
// Bullets

#[derive(PartialEq, Clone, Copy)]
struct BulletSpec<'a> {
    sprite: &'a Sprite<'a>,
    vel: f32,
    lifetime: f32,
    bbox: &'a BBox<'a>,
}

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
struct Bullet {
    spec: SpecId,
    trans: Transform,
    age: f32,
}

impl Bullet {
    fn advance<'a>(&self, sspec: &GameSpec<'a>, _: &mut Actors, dt: f32) -> Option<Bullet> {
        let spec = sspec.specs[self.spec].is_bullet();
        let pos = Vec2 {
            x: self.trans.pos.x + (spec.vel * self.trans.rotation.cos() * dt),
            y: self.trans.pos.y + (-1. * spec.vel * self.trans.rotation.sin() * dt),
        };
        let bullet = Bullet {
            spec: self.spec,
            trans: Transform{pos: pos, rotation: self.trans.rotation},
            age: self.age + dt,
        };
        let alive =
            bullet.trans.pos.x >= 0. && bullet.trans.pos.x <= sspec.map.w &&
            bullet.trans.pos.y >= 0. && bullet.trans.pos.y <= sspec.map.h &&
            bullet.age < spec.lifetime;
        if alive { Some(bullet) } else { None }
    }

    fn render<'a>(&self, sspec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        let spec = sspec.specs[self.spec].is_bullet();
        let trans = trans.adjust(&self.trans);
        try!(spec.sprite.render(renderer, &trans));
        // Debugging -- render bbox
        spec.bbox.render(renderer, &trans)
    }
}

// ---------------------------------------------------------------------
// Ship

#[derive(PartialEq, Clone, Copy)]
struct ShipSpec<'a> {
    rotation_vel: f32,
    rotation_vel_accel: f32,
    accel: f32,
    friction: f32,
    gravity: f32,
    sprite: &'a Sprite<'a>,
    sprite_accel: &'a Sprite<'a>,
    bullet_spec: SpecId,
    firing_interval: f32,
    shoot_from: Vec2,
    bbox: &'a BBox<'a>,
}

#[derive(PartialEq, Clone, Show, Copy)]
struct CameraSpec {
    accel: f32,
    // The minimum distance from the top/bottom edges to the ship
    v_pad: f32,
    // The minimum distance from the left/right edges to the ship
    h_pad: f32,
}

#[derive(PartialEq, Clone, Show, Copy, RustcEncodable, RustcDecodable)]
struct Camera {
    pos: Vec2,
    vel: Vec2,
}

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
struct Ship {
    spec: SpecId,
    trans: Transform,
    vel: Vec2,
    not_firing_for: f32,
    accel: bool,
    rotating: Rotating,
    camera: Camera,
}

struct ShipState<'a> {
    spec: &'a ShipSpec<'a>,
    accel: bool,
    rotation: f32,
}

impl<'a> physics::Acceleration for ShipState<'a> {
    fn accel(&self, state: &physics::State) -> Vec2 {
        let mut f = Vec2::zero();
        // Acceleration
        if self.accel {
            f.x += self.rotation.cos() * self.spec.accel;
            // The sin is inverted because we push the opposite
            // direction we're looking at.
            f.y += -1. * self.rotation.sin() * self.spec.accel;
        }

        // Gravity
        f.y += self.spec.gravity;

        // Friction
        let friction = state.v * self.spec.friction;
        f = f - friction;

        // Done
        f
    }
}

impl Camera {
    #[inline]
    fn transform(&self) -> Transform {
        Transform{pos: self.pos, rotation: 0.}
    }

    #[inline(always)]
    fn left(&self) -> f32 { self.pos.x }
    #[inline(always)]
    fn right(&self) -> f32 { self.pos.x + SCREEN_WIDTH }
    #[inline(always)]
    fn top(&self) -> f32 { self.pos.y }
    #[inline(always)]
    fn bottom(&self) -> f32 { self.pos.y + SCREEN_HEIGHT }

    #[inline]
    fn advance<'a>(&self, sspec: &GameSpec<'a>, ship_vel: Vec2, ship_trans: Transform, dt: f32) -> Camera {
        let &mut cam = self;
        let spec = sspec.camera_spec;
        let map = sspec.map;

        // Push the camera based on the ship vel
        cam.vel = ship_vel * spec.accel;
        cam.pos = cam.pos + cam.vel * dt;

        // Make sure the ship is not too much to the edge
        if cam.left() + spec.h_pad > ship_trans.pos.x {
            cam.pos.x = ship_trans.pos.x - spec.h_pad
        } else if cam.right() - spec.h_pad < ship_trans.pos.x {
            cam.pos.x = (ship_trans.pos.x + spec.h_pad) - SCREEN_WIDTH
        }
        if cam.top() + spec.v_pad > ship_trans.pos.y {
            cam.pos.y = ship_trans.pos.y - spec.v_pad
        } else if cam.bottom() - spec.v_pad < ship_trans.pos.y {
            cam.pos.y = (ship_trans.pos.y + spec.v_pad) - SCREEN_HEIGHT
        }

        // Make sure it stays in the map
        cam.pos = map.bound_rect(cam.pos, SCREEN_WIDTH, SCREEN_HEIGHT);

        cam
    }
}

impl Ship {
    fn new(spec_id: SpecId, pos: Vec2) -> Ship {
        Ship{
            spec: spec_id,
            trans: Transform::pos(pos),
            vel: Vec2::zero(),
            not_firing_for: 100000.,
            accel: false,
            rotating: Rotating::Still,
            camera: Camera{
                pos: Vec2{
                    x: pos.x - SCREEN_WIDTH/2.,
                    y: pos.y - SCREEN_HEIGHT/2.,
                },
                vel: Vec2::zero(),
            }
        }
    }

    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, input: Option<Input>, dt: f32) -> Option<Ship> {
        let spec = sspec.specs[self.spec].is_ship();
        let mut not_firing_for = self.not_firing_for + dt;
        let (accel, rotating, firing) =
            match input {
                None => (self.accel, self.rotating, false),
                Some(input) => {
                    let firing = if input.firing && self.not_firing_for >= spec.firing_interval {
                        not_firing_for = 0.;
                        true
                    } else {
                        false
                    };
                    (input.accel, input.rotating, firing)
                },
            };
        let mut trans = self.trans;
        let mut vel = self.vel;

        // =============================================================
        // Apply the rotation
        let rotation_vel = if accel {
            spec.rotation_vel_accel
        } else {
            spec.rotation_vel
        };
        let rotation_delta = dt * rotation_vel;
        match rotating {
            Rotating::Still => {},
            Rotating::Left  => trans.rotation += rotation_delta,
            Rotating::Right => trans.rotation -= rotation_delta,
        }

        // =============================================================
        // Apply the force
        let st = physics::State {pos: trans.pos, v: vel};
        let st = physics::integrate(&ShipState{spec: spec, accel: accel, rotation: trans.rotation}, &st, dt);
        vel = st.v;
        trans.pos = st.pos;
        trans.pos = sspec.map.bound(trans.pos);

        // =============================================================
        // Move the camera
        let camera = self.camera.advance(sspec, vel, trans, dt);

        // =============================================================
        // Add new bullet
        if firing {
            let shoot_from = spec.shoot_from.rotate(trans.rotation);
            let bullet = Bullet {
                spec: spec.bullet_spec,
                trans: trans + shoot_from,
                age: 0.,
            };
            actors.add(Actor::Bullet(bullet));
        }
        
        let new = Ship {
            spec: self.spec,
            trans: trans,
            vel: vel,
            not_firing_for: not_firing_for,
            accel: accel,
            rotating: rotating,
            camera: camera,
        };
        Some(new)
    }

    fn render<'a>(&self, sspec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        let trans = trans.adjust(&self.trans);
        let spec = sspec.specs[self.spec].is_ship();

        // =============================================================
        // Render ship
        if self.accel {
            try!(spec.sprite_accel.render(renderer, &trans));
        } else {
            try!(spec.sprite.render(renderer, &trans));
        }

        // =============================================================
        // Debugging -- render bbox
        spec.bbox.render(renderer, &trans)
    }
}

// ---------------------------------------------------------------------
// Shooter

#[derive(PartialEq, Clone, Copy)]
struct ShooterSpec<'a> {
    sprite: &'a Sprite<'a>,
    trans: Transform,
    bullet_spec: SpecId,
    firing_rate: f32,
}

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
struct Shooter {
    spec: SpecId,
    time_since_fire: f32,
}

impl Shooter {
    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, dt: f32) -> Option<Shooter> {
        let spec = sspec.specs[self.spec].is_shooter();
        let mut time_since_fire = self.time_since_fire + dt;
        if time_since_fire > spec.firing_rate {
            time_since_fire = 0.;
            let bullet = Bullet {
                spec: spec.bullet_spec,
                trans: spec.trans,
                age: 0.,
            };
            actors.add(Actor::Bullet(bullet));
        }
        Some(Shooter{spec: self.spec, time_since_fire: time_since_fire})
    }

    fn render<'a>(&self, sspec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        let spec = sspec.specs[self.spec].is_shooter();
        spec.sprite.render(renderer, &trans.adjust(&spec.trans))
    }
}

// ---------------------------------------------------------------------
// Game

#[derive(PartialEq, Clone, Copy)]
struct GameSpec<'a> {
    map: &'a Map<'a>,
    camera_spec: &'a CameraSpec,
    ship_spec: SpecId,
    shooter_spec: SpecId,
    specs: &'a [Spec<'a>],
}

#[derive(PartialEq, Clone, Show)]
struct Actors {
    actors: HashMap<ActorId, Actor>,
    count: ActorId,
}

impl<E, S: Encoder<E>> Encodable<S, E> for Actors {
    fn encode(&self, s: &mut S) -> Result<(), E> {
        for pair in self.actors.iter() {
            try!(pair.encode(s));
        }
        self.count.encode(s)
    }
}

impl Actors {
    fn new() -> Actors {
        Actors{actors: HashMap::new(), count: 0}
    }

    fn prepare_new(old: &Actors) -> Actors {
        Actors{
            actors: HashMap::with_capacity(old.actors.len()),
            count: old.count,
        }
    }

    fn add(&mut self, actor: Actor) -> ActorId {
        let actor_id = self.count;
        self.count += 1;
        self.actors.insert(actor_id, actor);
        actor_id
    }

    fn insert(&mut self, actor_id: ActorId, actor: Actor) {
        self.actors.insert(actor_id, actor);
    }

    fn get(&self, actor_id: ActorId) -> &Actor {
        match self.actors.get(&actor_id) {
            None => unreachable!(),
            Some(actor) => actor,
        }
    }
}

#[derive(PartialEq, Clone, Show, RustcEncodable)]
struct Game {
    actors: Actors,
}

// ---------------------------------------------------------------------
// Input

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
struct Input {
    quit: bool,
    accel: bool,
    firing: bool,
    rotating: Rotating,
    paused: bool,
}

impl Input {
    fn process_events(&mut self) {
        loop {
            match sdl2::event::poll_event() {
                sdl2::event::Event::None =>
                    break,
                sdl2::event::Event::Quit(_) =>
                    self.quit = true,
                sdl2::event::Event::KeyDown(_, _, key, _, _, _) =>
                    match key {
                        sdl2::keycode::KeyCode::Left  => self.rotating = Rotating::Left,
                        sdl2::keycode::KeyCode::Right => self.rotating = Rotating::Right,
                        sdl2::keycode::KeyCode::Up    => self.accel = true,
                        sdl2::keycode::KeyCode::X     => self.firing = true,
                        sdl2::keycode::KeyCode::P     => self.paused = !self.paused,
                        _                             => {},
                    },
                sdl2::event::Event::KeyUp(_, _, key, _, _, _) => {
                    if self.accel && key == sdl2::keycode::KeyCode::Up {
                        self.accel = false
                    };
                    if self.firing && key == sdl2::keycode::KeyCode::X {
                        self.firing = false;
                    };
                    if self.rotating == Rotating::Left && key == sdl2::keycode::KeyCode::Left {
                        self.rotating = Rotating::Still;
                    };
                    if self.rotating == Rotating::Right && key == sdl2::keycode::KeyCode::Right {
                        self.rotating = Rotating::Still;
                    };
                },
                _ => {},
            }
        };
    }
}

#[derive(PartialEq, Clone, Copy)]
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
    fn advance<'a>(&self, spec: &GameSpec<'a>, inputs: &Vec<ShipInput>, dt: f32) -> Game {
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

    fn render<'a>(&self, spec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        // Paint the background for the whole thing
        try!(renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)));
        try!(spec.map.render(renderer, &trans.pos));
        for actor in self.actors.actors.values() {
            try!(actor.render(spec, renderer, trans));
        };
        Ok(())
    }

    fn add_ship<'a>(&mut self, spec: &GameSpec<'a>) -> ActorId {
        let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
        self.actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)))
    }
}

// ---------------------------------------------------------------------
// Server

type SnapshotId = usize;

#[derive(PartialEq, Clone)]
struct Server<'a> {
    game_spec: &'a GameSpec<'a>,
    game: Game,
    clients: HashMap<SocketAddr, ActorId>,
}

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

    fn worker(queue: Sender<Message>, server_mutex: Arc<Mutex<network::Server>>) -> ! {
        loop {
            println!("Worker looping");
            let (addr, input): (SocketAddr, IoResult<Input>) = {
                let mut server = server_mutex.lock().unwrap();
                server.recv().ok().expect("Server.worker: Could not receive")
            };
            match input {
                Err(err) =>
                    println!("Server.worker: couldn't decode message: {}", err),
                Ok(input) => {
                    let msg = Message{from: addr, input: input};
                    queue.send(msg)
                }
            }
        }
    }

    // FIXME: static bound on the messages we can get, to keep up with
    // network
    fn drain(queue: &Receiver<Message>) -> Vec<Message> {
        let mut vec = Vec::new();
        loop {
            let msg = queue.try_recv();
            match msg {
                // FIXME: handle disconnections
                Err(_)  => break,
                Ok(msg) => vec.push(msg),
            }
        };
        vec
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

    fn broadcast_game(&self, server_mutex: Arc<Mutex<network::Server>>) -> IoResult<()> {
        // FIXME: encode once
        let mut server = server_mutex.lock().unwrap();
        for addr in self.clients.keys() {
            try!(server.send(*addr, &self.game));
        };
        Ok(())
    }

    fn run<A: ToSocketAddr>(self, addr: &A) {
        let addr = addr.to_socket_addr().ok().expect("Server.run: could not get SocketAddr");
        let server = network::Server::new(addr).ok().expect("Server.worker: Could not create network server");
        let server_local_mutex = Arc::new(Mutex::new(server));
        let server_remote_mutex = server_local_mutex.clone();
        let (tx, rx) = channel();
        let guard: JoinGuard<()> = Thread::spawn(move || {
            Server::worker(tx, server_remote_mutex)
        });
        guard.detach();

        let wait_ms = (TIME_STEP * 1000.) as usize;
        let mut state = self;
        loop {
            let quit = Server::should_quit();
            if quit { println!("Server quitting!"); break };

            let inputs = Server::drain(&rx);
            let inputs = state.prepare_inputs(&inputs);
            state.game = state.game.advance(state.game_spec, &inputs, TIME_STEP);
            state.broadcast_game(server_local_mutex.clone()).ok().expect("Couldn't broadcast messages");

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
// Remote client

#[derive(PartialEq, Clone)]
struct RemoteClient {
    game_spec: &'a GameSpec<'a>
}


// ---------------------------------------------------------------------
// Client

#[derive(PartialEq, Clone)]
struct Client<'a> {
    game_spec: &'a GameSpec<'a>,
    game: Game,
    player_id: ActorId,
}

impl<'a> Client<'a> {
    fn advance(&self, input: &Input, dt: f32) -> Client<'a> {
        let inputs = vec![ShipInput{ship: self.player_id, input: *input}];
        let game = self.game.advance(self.game_spec, &inputs, dt);
        Client{
            game_spec: self.game_spec,
            game: game,
            player_id: self.player_id
        }
    }

    fn run(self, renderer: &Renderer) {
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
                let mut current = previous.advance(&input, TIME_STEP);
                while accumulator >= TIME_STEP {
                    accumulator -= TIME_STEP;
                    let new = current.advance(&input, TIME_STEP);
                    previous = current;
                    current = new;
                }
                // TODO: interpolate previous and current
                state = current;
            }

            let camera = state.game.actors.get(state.player_id).is_ship().camera;
            state.game.render(state.game_spec, renderer, &camera.transform()).ok().expect("Failed to render the state");
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

        let client = Client{
            game_spec: spec,
            game: game,
            player_id: player_id,
        };
        client.run(&renderer);
    })
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
