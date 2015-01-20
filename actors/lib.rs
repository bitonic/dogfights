#![allow(unstable)]
extern crate "rustc-serialize" as rustc_serialize;

extern crate geometry;
extern crate input;
extern crate specs;
extern crate conf;
extern crate physics;

use std::collections::HashMap;
use std::collections::hash_map::{Keys, Values, Iter};
use std::num::Float;
use std::sync::Arc;
use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};

use geometry::*;
use input::*;
use specs::*;
use conf::*;

#[derive(PartialEq, Clone, Show, Copy, RustcEncodable, RustcDecodable)]
pub struct Camera {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl Camera {
    #[inline]
    pub fn transform(&self) -> Transform {
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
    pub fn advance(self, sspec: &GameSpec, ship_vel: Vec2, ship_trans: Transform, dt: f32) -> Camera {
        let mut cam = self;
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

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
pub struct Bullet {
    pub spec: SpecId,
    pub trans: Transform,
    pub age: f32,
}

impl Bullet {
    pub fn advance(&self, sspec: &GameSpec, _: &mut Actors, dt: f32) -> Option<Bullet> {
        let spec = sspec.get_spec(self.spec).is_bullet();
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
}


#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
pub struct Ship {
    pub spec: SpecId,
    pub trans: Transform,
    pub vel: Vec2,
    pub not_firing_for: f32,
    pub accel: bool,
    pub rotating: Rotating,
    pub camera: Camera,
}

struct ShipState<'a> {
    spec: &'a ShipSpec,
    accel: bool,
    rotation: f32,
}

impl<'a> ::physics::Acceleration for ShipState<'a> {
    fn accel(&self, state: &::physics::State) -> Vec2 {
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
        let friction = state.vel * self.spec.friction;
        f = f - friction;

        // Done
        f
    }
}

impl Ship {
    pub fn new(spec_id: SpecId, pos: Vec2) -> Ship {
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

    pub fn advance(&self, sspec: &GameSpec, actors: &mut Actors, input: Option<Input>, dt: f32) -> Option<Ship> {
        let spec = sspec.get_spec(self.spec).is_ship();
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
        let st = ::physics::State {pos: trans.pos, vel: vel};
        let st = ::physics::integrate(&ShipState{spec: spec, accel: accel, rotation: trans.rotation}, &st, dt);
        vel = st.vel;
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
            let _ = actors.add(Actor::Bullet(bullet));
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
}

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
pub struct Shooter {
    pub spec: SpecId,
    pub time_since_fire: f32,
}

impl Shooter {
    pub fn advance(&self, sspec: &GameSpec, actors: &mut Actors, dt: f32) -> Option<Shooter> {
        let spec = sspec.get_spec(self.spec).is_shooter();
        let mut time_since_fire = self.time_since_fire + dt;
        if time_since_fire > spec.firing_rate {
            time_since_fire = 0.;
            let bullet = Bullet {
                spec: spec.bullet_spec,
                trans: spec.trans,
                age: 0.,
            };
            let _ = actors.add(Actor::Bullet(bullet));
        }
        Some(Shooter{spec: self.spec, time_since_fire: time_since_fire})
    }
}

// FIXME: efficient serialization using u8
#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
pub enum Actor {
    Ship(Ship),
    Shooter(Shooter),
    Bullet(Bullet),
}

impl Actor {
    // Returns whether the actor is still alive
    pub fn advance(&self, sspec: &GameSpec, actors: &mut Actors, input: Option<Input>, dt: f32) -> Option<Actor> {
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

    pub fn interact(&self, _: &GameSpec, _: &Actors) -> Option<Actor> {
        Some(*self)
    }

    pub fn is_ship(&self) -> &Ship {
        match *self {
            Actor::Ship(ref ship) => ship,
            _                     => unreachable!(),
        }
    }
}

pub type ActorId = u32;

#[derive(PartialEq, Clone, Show)]
pub struct Actors {
    actors: HashMap<ActorId, Actor>,
    count: ActorId,
}

impl Encodable for Actors {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        let len: u32 = self.actors.len() as u32;
        try!(len.encode(s));
        for pair in self.actors.iter() {
            try!(pair.encode(s));
        }
        self.count.encode(s)
    }
}

impl Decodable for Actors {
    fn decode<D: Decoder>(d: &mut D) -> Result<Actors, D::Error> {
        let len: u32 = try!(Decodable::decode(d));
        let len: usize = len as usize;
        let mut actors = HashMap::new();
        for _ in range(0, len) {
            let (actor_id, actor) = try!(Decodable::decode(d));
            let _ = actors.insert(actor_id, actor);
        }
        let count = try!(Decodable::decode(d));
        Ok(Actors{actors: actors, count: count})
    }
}

impl Actors {
    pub fn new() -> Actors {
        Actors{actors: HashMap::new(), count: 0}
    }

    pub fn prepare_new(old: &Actors) -> Actors {
        Actors{
            actors: HashMap::with_capacity(old.actors.len()),
            count: old.count,
        }
    }

    pub fn add(&mut self, actor: Actor) -> ActorId {
        let actor_id = self.count;
        self.count += 1;
        self.insert(actor_id, actor);
        actor_id
    }

    pub fn remove(&mut self, actor_id: ActorId) -> Option<Actor> {
        self.actors.remove(&actor_id)
    }

    pub fn insert(&mut self, actor_id: ActorId, actor: Actor) {
        let _ = self.actors.insert(actor_id, actor);
    }

    pub fn get(&self, actor_id: ActorId) -> Option<&Actor> {
        self.actors.get(&actor_id)
    }

    pub fn keys(&self) -> Keys<ActorId, Actor> {
        self.actors.keys()
    }

    pub fn values(&self) -> Values<ActorId, Actor> {
        self.actors.values()
    }

    pub fn iter(&self) -> Iter<ActorId, Actor> {
        self.actors.iter()
    }

    pub fn len(&self) -> usize {
        self.actors.len()
    }
}

#[derive(PartialEq, Clone, Copy, Show)]
pub struct PlayerInput {
    pub player: ActorId,
    pub input: Input,
}

impl PlayerInput {
    pub fn lookup(inputs: &Vec<PlayerInput>, actor_id: ActorId) -> Option<Input> {
        for input in inputs.iter() {
            if input.player == actor_id { return Some(input.input) }
        };
        None
    }
}

#[derive(PartialEq, Clone, Show, RustcEncodable, RustcDecodable)]
pub struct Game {
    pub actors: Actors,
    pub time: f32,
}

impl Game {
    pub fn new() -> Game {
        Game{actors: Actors::new(), time: 0.}
    }

    pub fn advance(&self, spec: &GameSpec, inputs: &Vec<PlayerInput>, dt: f32) -> Game {
        // First move everything, spawn new stuff
        let mut advanced_actors = Actors::prepare_new(&self.actors);
        for (actor_id, actor) in self.actors.iter() {
            let actor_input = PlayerInput::lookup(inputs, *actor_id);
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

    pub fn add_ship(&mut self, spec: &GameSpec) -> ActorId {
        let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
        self.actors.add(Actor::Ship(Ship::new(spec.ship_spec, ship_pos)))
    }
}

#[derive(Clone, RustcEncodable, RustcDecodable)]
pub struct PlayerGame {
    pub player: ActorId,
    pub game: Arc<Game>,
}

impl PlayerGame {
    pub fn advance(&self, spec: &GameSpec, input: Input, dt: f32) -> PlayerGame {
        let inputs: Vec<PlayerInput> = vec![
            PlayerInput{
                player: self.player,
                input: input,
            }];
        let game = Arc::new(self.game.advance(spec, &inputs, dt));
        PlayerGame{
            game: game,
            player: self.player
        }
    }
}
