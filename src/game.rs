extern crate sdl2;
extern crate sdl2_image;

use std::num::FloatMath;
use std::collections::HashMap;
use std::collections::hash_map::{Entries, Values};
use sdl2::SdlResult;
use sdl2::pixels::Color;
use sdl2::render::{Renderer, Texture};
use geometry::{to_radians, from_radians, Vec2, Rect, Transform};
use physics;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: f64 = 800.;
static SCREEN_HEIGHT: f64 = 600.;

// 50 ms timesteps
const TIME_STEP: f64 = 0.01;
const MAX_FRAME_TIME: f64 = 0.250;

// ---------------------------------------------------------------------
// Bounding boxes

#[deriving(PartialEq, Clone)]
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

#[deriving(PartialEq, Clone, Copy)]
struct Sprite<'a> {
    texture: &'a Texture,
    rect: Rect,
    center: Vec2,
    // If the sprite is already rotated by some angle
    angle: f64,
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

#[deriving(PartialEq, Clone, Copy)]
struct Map<'a> {
    w: f64,
    h: f64,
    background_color: Color, 
    background_texture: &'a Texture,
}

impl<'a> Map<'a> {
    fn bound(&self, p: Vec2) -> Vec2 {
        // TODO handle points that are badly negative
        fn f(n: f64, m: f64) -> f64 {
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

    fn bound_rect(&self, p: Vec2, w: f64, h: f64) -> Vec2 {
        fn f(n: f64, edge: f64, m: f64) -> f64 {
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
        let bgr_w = bgr.width as f64;
        let bgr_h = bgr.height as f64;
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

type ActorId = uint;

#[deriving(PartialEq, Clone, Copy, Show)]
enum Actor {
    Ship(Ship),
    Shooter(Shooter),
    Bullet(Bullet),
}

impl Actor {
    // Returns whether the actor is still alive
    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, input: Option<Input>, dt: f64) -> Option<Actor> {
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
}

type SpecId = uint;

#[deriving(PartialEq, Clone, Copy)]
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

#[deriving(PartialEq, Clone, Copy)]
struct BulletSpec<'a> {
    sprite: &'a Sprite<'a>,
    velocity: f64,
    lifetime: f64,
    bbox: &'a BBox<'a>,
}

#[deriving(PartialEq, Clone, Copy, Show)]
struct Bullet {
    spec: SpecId,
    trans: Transform,
    age: f64,
}

impl Bullet {
    fn advance<'a>(&self, sspec: &GameSpec<'a>, _: &mut Actors, dt: f64) -> Option<Bullet> {
        let spec = sspec.specs[self.spec].is_bullet();
        let pos = Vec2 {
            x: self.trans.pos.x + (spec.velocity * self.trans.rotation.cos() * dt),
            y: self.trans.pos.y + (-1. * spec.velocity * self.trans.rotation.sin() * dt),
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

#[deriving(PartialEq, Clone, Copy)]
struct ShipSpec<'a> {
    rotation_velocity: f64,
    rotation_velocity_accelerating: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
    sprite: &'a Sprite<'a>,
    sprite_accelerating: &'a Sprite<'a>,
    bullet_spec: SpecId,
    firing_interval: f64,
    shoot_from: Vec2,
    bbox: &'a BBox<'a>,
}

#[deriving(PartialEq, Clone, Copy, Show)]
struct Ship {
    spec: SpecId,
    trans: Transform,
    velocity: Vec2,
    not_firing_for: f64,
    accelerating: bool,
    rotating: Rotating,
}

struct ShipState<'a> {
    spec: &'a ShipSpec<'a>,
    accelerating: bool,
    rotation: f64,
}

impl<'a> physics::Acceleration for ShipState<'a> {
    fn acceleration(&self, state: &physics::State) -> Vec2 {
        let mut f = Vec2::zero();
        // Acceleration
        if self.accelerating {
            f.x += self.rotation.cos() * self.spec.acceleration;
            // The sin is inverted because we push the opposite
            // direction we're looking at.
            f.y += -1. * self.rotation.sin() * self.spec.acceleration;
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

impl Ship {
    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, input: Option<Input>, dt: f64) -> Option<Ship> {
        let spec = sspec.specs[self.spec].is_ship();
        let mut not_firing_for = self.not_firing_for + dt;
        let (accelerating, rotating, firing) =
            match input {
                None => (self.accelerating, self.rotating, false),
                Some(input) => {
                    let firing = if input.firing && self.not_firing_for >= spec.firing_interval {
                        not_firing_for = 0.;
                        true
                    } else {
                        false
                    };
                    (input.accelerating, input.rotating, firing)
                },
            };
        let mut trans = self.trans;
        let mut velocity = self.velocity;

        // =============================================================
        // Apply the rotation
        let rotation_velocity = if accelerating {
            spec.rotation_velocity_accelerating
        } else {
            spec.rotation_velocity
        };
        let rotation_delta = dt * rotation_velocity;
        match rotating {
            Rotating::Still => {},
            Rotating::Left  => trans.rotation += rotation_delta,
            Rotating::Right => trans.rotation -= rotation_delta,
        }

        // =============================================================
        // Apply the force
        let st = physics::State {pos: trans.pos, v: velocity};
        let st = physics::integrate(&ShipState{spec: spec, accelerating: accelerating, rotation: trans.rotation}, &st, dt);
        velocity = st.v;
        trans.pos = st.pos;
        trans.pos = sspec.map.bound(trans.pos);

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
            velocity: velocity,
            not_firing_for: not_firing_for,
            accelerating: accelerating,
            rotating: rotating,
        };
        Some(new)
    }

    fn render<'a>(&self, sspec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        let trans = trans.adjust(&self.trans);
        let spec = sspec.specs[self.spec].is_ship();

        // =============================================================
        // Render ship
        if self.accelerating {
            try!(spec.sprite_accelerating.render(renderer, &trans));
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

#[deriving(PartialEq, Clone, Copy)]
struct ShooterSpec<'a> {
    sprite: &'a Sprite<'a>,
    trans: Transform,
    bullet_spec: SpecId,
    firing_rate: f64,
}

#[deriving(PartialEq, Clone, Copy, Show)]
struct Shooter {
    spec: SpecId,
    time_since_fire: f64,
}

impl Shooter {
    fn advance<'a>(&self, sspec: &GameSpec<'a>, actors: &mut Actors, dt: f64) -> Option<Shooter> {
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

#[deriving(PartialEq, Clone, Copy)]
struct GameSpec<'a> {
    map: &'a Map<'a>,
    specs: &'a [Spec<'a>],
}

#[deriving(PartialEq, Clone, Show)]
struct Actors {
    actors: HashMap<ActorId, Actor>,
    count: ActorId,
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

    fn iter(&self) -> Entries<ActorId, Actor> {
        self.actors.iter()
    }

    fn values(&self) -> Values<ActorId, Actor> {
        self.actors.values()
    }

    fn get(&self, actor_id: ActorId) -> &Actor {
        match self.actors.get(&actor_id) {
            None => unreachable!(),
            Some(actor) => actor,
        }
    }
}

#[deriving(PartialEq, Clone, Show)]
struct Game {
    actors: Actors,
}

// ---------------------------------------------------------------------
// Input

#[deriving(PartialEq, Clone, Copy, Show)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq, Clone, Copy)]
struct Input {
    quit: bool,
    accelerating: bool,
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
                        sdl2::keycode::KeyCode::Up    => self.accelerating = true,
                        sdl2::keycode::KeyCode::X     => self.firing = true,
                        sdl2::keycode::KeyCode::P     => self.paused = !self.paused,
                        _                             => {},
                    },
                sdl2::event::Event::KeyUp(_, _, key, _, _, _) => {
                    if self.accelerating && key == sdl2::keycode::KeyCode::Up {
                        self.accelerating = false
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

#[deriving(PartialEq, Clone, Copy)]
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
    fn advance<'a>(&self, spec: &GameSpec<'a>, inputs: &Vec<ShipInput>, dt: f64) -> Game {
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
        }
    }

    fn render<'a>(&self, spec: &GameSpec<'a>, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
        // Paint the background for the whole thing
        try!(renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)));
        try!(spec.map.render(renderer, &trans.pos));
        for actor in self.actors.values() {
            try!(actor.render(spec, renderer, trans));
        };
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Camera

#[deriving(PartialEq, Clone, Show, Copy)]
struct CameraSpec {
    acceleration: f64,
    // The minimum distance from the top/bottom edges to the ship
    v_padding: f64,
    // The minimum distance from the left/right edges to the ship
    h_padding: f64,
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct Camera<'a> {
    spec: &'a CameraSpec,
    pos: Vec2,
    velocity: Vec2,
}

impl<'a> Camera<'a> {
    #[inline]
    fn transform(&self) -> Transform {
        Transform{pos: self.pos, rotation: 0.}
    }

    #[inline(always)]
    fn left(&self) -> f64 { self.pos.x }
    #[inline(always)]
    fn right(&self) -> f64 { self.pos.x + SCREEN_WIDTH }
    #[inline(always)]
    fn top(&self) -> f64 { self.pos.y }
    #[inline(always)]
    fn bottom(&self) -> f64 { self.pos.y + SCREEN_HEIGHT }

    fn advance(&self, map: &Map, ship: &Ship, dt: f64) -> Camera<'a> {
        let &mut cam = self;

        // Push the camera based on the ship velocity
        cam.velocity = ship.velocity * self.spec.acceleration;
        cam.pos = cam.pos + cam.velocity * dt;

        // Make sure the ship is not too much to the edge
        if cam.left() + cam.spec.h_padding > ship.trans.pos.x {
            cam.pos.x = ship.trans.pos.x - cam.spec.h_padding
        } else if cam.right() - cam.spec.h_padding < ship.trans.pos.x {
            cam.pos.x = (ship.trans.pos.x + cam.spec.h_padding) - SCREEN_WIDTH
        }
        if cam.top() + cam.spec.v_padding > ship.trans.pos.y {
            cam.pos.y = ship.trans.pos.y - cam.spec.v_padding
        } else if cam.bottom() - cam.spec.v_padding < ship.trans.pos.y {
            cam.pos.y = (ship.trans.pos.y + cam.spec.v_padding) - SCREEN_HEIGHT
        }

        // Make sure it stays in the map
        cam.pos = map.bound_rect(cam.pos, SCREEN_WIDTH, SCREEN_HEIGHT);

        cam
    }
}

// ---------------------------------------------------------------------
// Client

#[deriving(PartialEq, Clone)]
struct Client<'a> {
    game_spec: GameSpec<'a>,
    game: Game,
    camera: Camera<'a>,
    player_id: ActorId,
}

impl<'a> Client<'a> {
    fn advance(&self, input: &Input, dt: f64) -> Client<'a> {
        let inputs = vec![ShipInput{ship: self.player_id, input: *input}];
        let game = self.game.advance(&self.game_spec, &inputs, dt);
        let camera = {
            let ship = match *(game.actors.get(self.player_id)) {
                Actor::Ship(ref ship) => ship,
                _                     => unreachable!(),
            };
            self.camera.advance(self.game_spec.map, ship, dt)
        };
        Client{
            game_spec: self.game_spec,
            game: game,
            camera: camera,
            player_id: self.player_id
        }
    }

    fn run(self, renderer: &Renderer) {
        let mut prev_time = sdl2::get_ticks();
        let mut accumulator = 0.;
        let mut state = self;
        let mut input = Input{
            quit: false,
            accelerating: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        };
        loop {
            input.process_events();
            if input.quit { break };

            let time_now = sdl2::get_ticks();
            let frame_time = ((time_now - prev_time) as f64) / 1000.; // Seconds to millis
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

            state.game.render(&state.game_spec, renderer, &state.camera.transform()).ok().expect("Failed to render the state");
            renderer.present();
        }
    }
}

pub fn client() {
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

    // Specs
    let planes_surface = sdl2_image::LoadSurface::from_file(&("assets/planes.png".parse()).unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture: &Texture = &renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let mut specs = Vec::new();
    let bullet_spec = BulletSpec {
        sprite: &Sprite {
            texture: planes_texture,
            rect: Rect{pos: Vec2{x: 424., y: 140.}, w: 3., h: 12.},
            center: Vec2{x: 1., y: 6.},
            angle: 90.,
        },
        velocity: 1000.,
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
        rotation_velocity: 10.,
        rotation_velocity_accelerating: 1.,
        acceleration: 800.,
        friction: 0.02,
        gravity: 100.,
        sprite: &Sprite{
            texture: planes_texture,
            rect: Rect{pos: Vec2{x: 128., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        sprite_accelerating: &Sprite {
            texture: planes_texture,
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
            texture: planes_texture,
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
    let game_spec = GameSpec{
        map: map,
        specs: specs.as_slice(),
    };

    // Actors
    let mut actors = Actors::new();
    let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
    let player_id = actors.add(Actor::Ship(
        Ship{
            spec: ship_spec_id,
            trans: Transform::pos(ship_pos),
            velocity: Vec2::zero(),
            not_firing_for: 100000.,
            accelerating: false,
            rotating: Rotating::Still,
        }));
    let _ = actors.add(Actor::Shooter(
        Shooter{
            spec: shooter_spec_id,
            time_since_fire: 0.,
        }));
    let game = Game{actors: actors};

    // Camera
    let camera = Camera{
        spec: &CameraSpec {
            acceleration: 1.2,
            h_padding: 220.,
            v_padding: 220. * SCREEN_HEIGHT / SCREEN_WIDTH,
        },
        pos: Vec2{
            x: ship_pos.x - SCREEN_WIDTH/2.,
            y: ship_pos.y - SCREEN_HEIGHT/2.,
        },
        velocity: Vec2::zero(),
    };

    let client = Client{
        game_spec: game_spec,
        game: game,
        camera: camera,
        player_id: player_id,
    };
    client.run(&renderer);
}
