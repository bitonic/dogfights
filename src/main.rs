extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use std::num::FloatMath;
use vec2::Vec2;

pub mod vec2;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: i32 = 1280;
static SCREEN_HEIGHT: i32 = 720;

// ---------------------------------------------------------------------
// Ship

#[deriving(PartialEq, Clone, Show, Copy)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq, Clone, Copy)]
struct ShipSpec {
    color: Color,
    height: f64,
    width: f64,
    rotation_speed: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
    // edge_repulsion: f64,
}

#[deriving(PartialEq, Clone, Copy)]
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

        // Update speed
        self.speed = self.speed + f;

        // Update position
        self.pos.x += (self.speed.x * dt) as i32;
        self.pos.y += (self.speed.y * dt) as i32;
        self.pos = map.bound(self.pos);
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

#[deriving(PartialEq)]
struct Map {
    w: i32,
    h: i32,
    background_color: Color, 
   background_texture: sdl2::render::Texture,
}

impl Map {
    fn render(&self, renderer: &sdl2::render::Renderer, cam: &Camera) -> () {
        // Fill the whole screen with the background color
        renderer.set_draw_color(self.background_color).ok().unwrap();
        renderer.fill_rect(&Rect {x: 0, y: 0, w: SCREEN_WIDTH, h: SCREEN_HEIGHT}).ok().unwrap();

        // ┌──────────────────────────────────────────┐
        // │                  ┊                   ┊   │
        // │  cam.pos         ┊                   ┊   │
        // │  ┌─────────────────────┐             ┊   │
        // │  │               ┊     │             ┊   │
        // │  │             t ┊     │             ┊   │
        // │┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│
        // │  │               ┊     │             ┊   │
        // │  └─────────────────────┘             ┊   │
        // │                  ┊                   ┊   │
        // └──────────────────────────────────────────┘

        let bgr = self.background_texture.query().ok().unwrap();
        let bgr_w = bgr.width as i32;
        let bgr_h = bgr.height as i32;
        let t = Vec2 {
            x: bgr_w - (cam.pos.x % bgr_w),
            y: bgr_h - (cam.pos.y % bgr_h),
        };
        let top_left = Rect {
            x: t.x - bgr_w,
            y: t.y - bgr_h,
            w: bgr_w,
            h: bgr_h,
        };
        let top_right = Rect {
            x: t.x,
            y: t.y - bgr_h,
            .. top_left
        };
        let bottom_left = Rect {
            x: t.x - bgr_w,
            y: t.y,
            .. top_left
        };
        let bottom_right = Rect {
            x: t.x,
            y: t.y,
            .. top_left
        };
        
        renderer.copy(&self.background_texture, None, Some(top_left)).ok().unwrap();
        renderer.copy(&self.background_texture, None, Some(top_right)).ok().unwrap();
        renderer.copy(&self.background_texture, None, Some(bottom_left)).ok().unwrap();
        renderer.copy(&self.background_texture, None, Some(bottom_right)).ok().unwrap();
    }

    fn bound(&self, p: Vec2<i32>) -> Vec2<i32> {
        // TODO handle points that are badly negative
        fn f(n: i32, m: i32) -> i32 {
            if n < 0 {
                0
            } else if n > m {
                m
            } else {
                n
            }
        };
        Vec2{ x: f(p.x, self.w), y: f(p.y, self.h) }
    }

    fn bound_rect(&self, p: Vec2<i32>, w: i32, h: i32) -> Vec2<i32> {
        fn f(n: i32, edge: i32, m: i32) -> i32 {
            if n < 0 {
                0
            } else if n + edge > m {
                m - edge
            } else {
                n
            }
        };
        Vec2{ x: f(p.x, w, self.w), y: f(p.y, h, self.h) }
    }
}

// ---------------------------------------------------------------------
// Main

#[deriving(PartialEq, Clone, Show, Copy)]
struct Camera {
    acceleration: f64,
    // The minimum distance from the top/bottom edges to the ship
    v_padding: i32,
    // The minimum distance from the left/right edges to the ship
    h_padding: i32,
    pos: Vec2<i32>,
}

impl Camera {
    fn new(ship: &Ship, acceleration: f64, h_padding: i32) -> Camera {
        Camera {
            acceleration: acceleration,
            h_padding: h_padding,
            v_padding: h_padding * SCREEN_HEIGHT / SCREEN_WIDTH,
            pos: Vec2{
                x: ship.pos.x - SCREEN_WIDTH/2,
                y: ship.pos.y - SCREEN_HEIGHT/2,
            }
        }
    }

    fn adjust(&self, p: Vec2<i32>) -> Point {
        (p - self.pos).point()
    }

    #[inline(always)]
    fn left(&self) -> i32 { self.pos.x }
    #[inline(always)]
    fn right(&self) -> i32 { self.pos.x + SCREEN_WIDTH }
    #[inline(always)]
    fn top(&self) -> i32 { self.pos.y }
    #[inline(always)]
    fn bottom(&self) -> i32 { self.pos.y + SCREEN_HEIGHT }

    fn advance(&mut self, map: &Map, ship: &Ship, dt: f64) {
        // Push the camera based on the ship velocity
        let f = ship.speed * self.acceleration;
        // // But also centering the player
        // let target = Vec2{x: ship.pos.x - SCREEN_WIDTH/2, y: ship.pos.y - SCREEN_HEIGHT/2};
        // let towards = self.pos - target;
        // let f = f + Vec2{x: towards.x as f64, y: towards.y as f64} * self.chasing_speed;
        
        self.pos.x += (f.x * dt) as i32;
        self.pos.y += (f.y * dt) as i32;


        // Make sure the ship is not too much to the edge
        if self.left() + self.h_padding > ship.pos.x {
            self.pos.x = ship.pos.x - self.h_padding
        } else if self.right() - self.h_padding < ship.pos.x {
            self.pos.x = (ship.pos.x + self.h_padding) - SCREEN_WIDTH
        }
        if self.top() + self.v_padding > ship.pos.y {
            self.pos.y = ship.pos.y - self.v_padding
        } else if self.bottom() - self.v_padding < ship.pos.y {
            self.pos.y = (ship.pos.y + self.v_padding) - SCREEN_HEIGHT
        }

        // Make sure it stays in the map
        self.pos = map.bound_rect(self.pos, SCREEN_WIDTH, SCREEN_HEIGHT);
    }
}

#[deriving(PartialEq)]
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

        println!("Ship position: {}", state.ship.pos);
        println!("Camera position: {}", state.camera.pos);
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
            height: 30.,
            width: 20.,
            rotation_speed: 0.005,
            acceleration: 0.035,
            friction: 0.02,
            gravity: 0.008,
        },
        pos: Vec2 {x: SCREEN_WIDTH / 2, y: SCREEN_HEIGHT / 2},
        speed: Vec2 {x: 0., y: 0.},
        rotation: 0.,
    };
    let map_surface =
        sdl2::surface::Surface::from_bmp(&from_str("assets/background.bmp").unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let map = Map {
        w: SCREEN_WIDTH*10,
        h: SCREEN_HEIGHT*10,
        background_color: Color::RGB(0x58, 0xB7, 0xFF),
        background_texture: map_texture,
    };
    let mut state = State {
        quit: false,
        accelerating: false,
        rotating: Rotating::Still,
        time_delta: 0.,
        ship: ship,
        map: map,
        camera: Camera::new(&ship, 1.1, 300),
    };
    run(&renderer, &mut state, sdl2::get_ticks());
}
