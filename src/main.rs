extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use std::num::FloatMath;
use vec2::Vec2;

pub mod vec2;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: i32 = 1024;
static SCREEN_HEIGHT: i32 = 576;

// ---------------------------------------------------------------------
// Ship

#[deriving(PartialEq, Clone, Show, Copy)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct ShipSpec {
    color: Color,
    height: f64,
    width: f64,
    rotation_speed: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct Ship {
    spec: ShipSpec,
    pos: Vec2<i32>,
    speed: Vec2<f64>,
    rotation: f64,
}

impl Ship {
    fn advance(&mut self, map: &Map, accelerating: bool, rotating: Rotating, dt: f64) -> () {
        // =============================================================
        // Apply the rotation
        let rotation_delta = dt * self.spec.rotation_speed;
        match rotating {
            Rotating::Still => {},
            Rotating::Left  => self.rotation -= rotation_delta,
            Rotating::Right => self.rotation += rotation_delta,
        }

        // =============================================================
        // Apply the force
        let mut f = Vec2 {x : 0., y: 0.};
        // Acceleration
        if accelerating {
            f.x += self.rotation.cos() * self.spec.acceleration;
            f.y += self.rotation.sin() * self.spec.acceleration;
        }

        // Gravity
        f.y += self.spec.gravity;

        // Friction
        let friction = self.speed * self.spec.friction;
        f = f - friction;

        // Scale based on the time
        f = f * dt;

        // Update speed
        self.speed = self.speed + f;

        // Update position
        self.pos.x += self.speed.x as i32;
        self.pos.y += self.speed.y as i32;
        if self.pos.x < 0 {
            self.pos.x = 0;
        } else if self.pos.x > map.w {
            self.pos.x = map.w
        }
        if self.pos.y < 0 {
            self.pos.y = 0;
        } else if self.pos.y > map.h {
            self.pos.y = map.h
        }
    }

    fn render(&self, renderer: &sdl2::render::Renderer, cam: &Camera) -> () {
        renderer.set_draw_color(self.spec.color).ok().unwrap();

        let xf = self.pos.x as f64;
        let yf = self.pos.y as f64;
        let tip = Vec2 {
            x: (xf + self.rotation.cos() * self.spec.height) as i32,
            y: (yf + self.rotation.sin() * self.spec.height) as i32,
        };
        let ortogonal = self.rotation + 1.57079633;
        let base_l = Vec2 {
            x: (xf + ortogonal.cos() * self.spec.width/2.) as i32,
            y: (yf + ortogonal.sin() * self.spec.width/2.) as i32,
        };
        let base_r = Vec2 {
            x: (xf - ortogonal.cos() * self.spec.width/2.) as i32,
            y: (yf - ortogonal.sin() * self.spec.width/2.) as i32,
        };

        renderer.draw_line(cam.adjust(tip), cam.adjust(base_l)).ok().unwrap();
        renderer.draw_line(cam.adjust(tip), cam.adjust(base_r)).ok().unwrap();
        renderer.draw_line(cam.adjust(base_l), cam.adjust(base_r)).ok().unwrap();
    }
}

// ---------------------------------------------------------------------
// Maps

#[deriving(PartialEq, Clone, Show, Copy)]
struct Map {
    w: i32,
    h: i32
}

impl Map {
    fn render(&self, _: &sdl2::render::Renderer, _: &Camera) -> () {
        
    }
}

// ---------------------------------------------------------------------
// Main

#[deriving(PartialEq, Clone, Show, Copy)]
struct Camera {
    acceleration: f64,
    pos: Vec2<i32>,
}

impl Camera {
    fn adjust(&self, p: Vec2<i32>) -> Point {
        (p - self.pos).point()
    }

    fn advance(&mut self, map: &Map, ship: &Ship, _: f64) {
        // Just center the ship
        self.pos = ship.pos - Vec2{x : SCREEN_WIDTH/2, y: SCREEN_HEIGHT/2};

        // Make sure it stays in
        if self.pos.x < 0 {
            self.pos.x = 0
        } else if self.pos.x > map.w - SCREEN_WIDTH {
            self.pos.x = map.w - SCREEN_WIDTH
        }
        if self.pos.y < 0 {
            self.pos.y = 0
        } else if self.pos.y > map.h - SCREEN_HEIGHT {
            self.pos.y = map.h - SCREEN_HEIGHT
        }
    }
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct State {
    quit: bool,
    accelerating: bool,
    rotating: Rotating,
    time_delta: f64,
    ship: Ship,
    map: Map,
    camera: Camera,
}

// Tells us if we need to quit, if we are accelerating, and the rotation.
fn process_events(state: &mut State) -> () {
    loop {
        match sdl2::event::poll_event() {
            sdl2::event::Event::None =>
                break,
            sdl2::event::Event::Quit(_) =>
                state.quit = true,
            sdl2::event::Event::KeyDown(_, _, key, _, _, _) =>
                match key {
                    sdl2::keycode::KeyCode::Left  => state.rotating = Rotating::Left,
                    sdl2::keycode::KeyCode::Right => state.rotating = Rotating::Right,
                    sdl2::keycode::KeyCode::Up    => state.accelerating = true,
                    _                             => {},
                },
            sdl2::event::Event::KeyUp(_, _, key, _, _, _) =>
                match (state.accelerating, state.rotating, key) {
                    (true, _, sdl2::keycode::KeyCode::Up) =>
                        state.accelerating = false,
                    (_, Rotating::Left, sdl2::keycode::KeyCode::Left) =>
                        state.rotating = Rotating::Still,
                    (_, Rotating::Right, sdl2::keycode::KeyCode::Right) =>
                        state.rotating = Rotating::Still,
                    _ =>
                        {},
                },
            _ =>
                {},
        }
    }
}

fn run(renderer: &sdl2::render::Renderer, state: &mut State, prev_time0: u32) {
    let mut prev_time = prev_time0;
    loop {
        let time_now = sdl2::get_ticks();
        state.time_delta = (time_now - prev_time) as f64;

        process_events(state);
        state.ship.advance(&state.map, state.accelerating, state.rotating, state.time_delta);
        state.camera.advance(&state.map, &state.ship, state.time_delta);

        // Paint the background for the whole thing
        renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)).ok().unwrap();
        renderer.clear().ok().unwrap();
        // Paint the actual background, so that we get black bars
        renderer.set_draw_color(Color::RGB(0xFF, 0xFF, 0xFF)).ok().unwrap();
        renderer.fill_rect(&Rect {x: 0, y: 0, w: SCREEN_WIDTH, h: SCREEN_HEIGHT}).ok().unwrap();
        // Paint the map
        state.map.render(renderer, &state.camera);
        // Paint the ship
        state.ship.render(renderer, &state.camera);
        // GO
        renderer.present();

        if state.quit {
            break;
        }
        prev_time = time_now;
    }
}

fn main() {
    sdl2::init(sdl2::INIT_VIDEO);      // TODO add expect
    let window = sdl2::video::Window::new(
        "Asteroidi",
        sdl2::video::WindowPos::PosUndefined, sdl2::video::WindowPos::PosUndefined,
        (SCREEN_WIDTH as int), (SCREEN_HEIGHT as int),
        sdl2::video::SHOWN).ok().unwrap();
    let renderer = sdl2::render::Renderer::from_window(
        window,
        sdl2::render::RenderDriverIndex::Auto,
        sdl2::render::ACCELERATED | sdl2::render::PRESENTVSYNC).ok().unwrap();
    renderer.set_logical_size((SCREEN_WIDTH as int), (SCREEN_HEIGHT as int)).ok().unwrap();
    let ship = Ship {
        spec : ShipSpec {
            color: Color::RGB(0x00, 0x00, 0x00),
            height: 50.,
            width: 40.,
            rotation_speed: 0.005,
            acceleration: 0.025,
            friction: 0.001,
            gravity: 0.008,
        },
        pos: Vec2 {x: SCREEN_WIDTH / 2, y: SCREEN_HEIGHT / 2},
        speed: Vec2 {x: 0., y: 0.},
        rotation: 0.,
    };
    let mut state = State {
        quit: false,
        accelerating: false,
        rotating: Rotating::Still,
        time_delta: 0.,
        ship: ship,
        map: Map {w: SCREEN_WIDTH*4, h: SCREEN_HEIGHT*3},
        camera: Camera{acceleration: 0.1, pos: Vec2{x: 0, y: 0}},
    };
    run(&renderer, &mut state, sdl2::get_ticks());
}
