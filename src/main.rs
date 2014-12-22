extern crate sdl2;
extern crate sdl2_image;

use sdl2::pixels::Color;
use sdl2::rect::{Rect, Point};
use sdl2::SdlResult;
use sdl2::render::{Renderer, Texture};
use std::num::FloatMath;
use vec2::Vec2;

pub mod vec2;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: i32 = 800;
static SCREEN_HEIGHT: i32 = 600;

// ---------------------------------------------------------------------
// Sprites

#[deriving(PartialEq)]
struct Sprite<'a> {
    texture: & 'a Texture,
    rect: Rect,
    center: Point,
    // If the sprite is already rotated by some angle
    angle: f64,
}

impl<'a> Sprite<'a> {
    fn render(&self, renderer: &Renderer, rotation: f64, dst: Option<Rect>) -> SdlResult<()> {
        let dst = match dst {
            None       => None,
            Some(rect) => Some(Rect{x: rect.x - self.center.x, y: rect.y - self.center.y, .. rect}),
        };
        let angle = rotation * 180./std::f64::consts::PI;
        renderer.copy_ex(
            self.texture, Some(self.rect), dst, self.angle - (-1. * angle),
            Some(self.center), sdl2::render::RendererFlip::None)
    }
}

// ---------------------------------------------------------------------
// Ship

#[deriving(PartialEq, Clone, Show, Copy)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq)]
struct ShipSpec<'a> {
    color: Color,
    height: f64,
    width: f64,
    rotation_speed: f64,
    rotation_speed_accelerating: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
    sprite: Sprite<'a>,
    sprite_accelerating: Sprite<'a>,
}

#[deriving(PartialEq)]
struct Ship<'a> {
    spec: ShipSpec<'a>,
    pos: Vec2<i32>,
    speed: Vec2<f64>,
    rotation: f64,
}

impl<'a> Ship<'a> {
    fn advance(&mut self, map: &Map, accelerating: bool, rotating: Rotating, dt: f64) -> () {
        // =============================================================
        // Apply the rotation
        let rotation_speed = if accelerating {
            self.spec.rotation_speed_accelerating
        } else {
            self.spec.rotation_speed
        };
        let rotation_delta = dt * rotation_speed;
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

    fn render(&self, renderer: &Renderer, accelerating: bool, cam: &Camera) -> () {
        let pos = cam.adjust(self.pos);
        let dst = Rect{x: pos.x, y: pos.y, .. self.spec.sprite.rect};
        if accelerating {
            self.spec.sprite_accelerating.render(renderer, self.rotation, Some(dst)).ok().unwrap()
        } else {
            self.spec.sprite.render(renderer, self.rotation, Some(dst)).ok().unwrap()
        }
    }
}

// ---------------------------------------------------------------------
// Maps

#[deriving(PartialEq)]
struct Map<'a> {
    w: i32,
    h: i32,
    background_color: Color, 
    background_texture: & 'a Texture,
}

impl<'a> Map<'a> {
    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        // Fill the whole screen with the background color
        renderer.set_draw_color(self.background_color).ok().unwrap();
        renderer.fill_rect(&Rect {x: 0, y: 0, w: SCREEN_WIDTH, h: SCREEN_HEIGHT}).ok().unwrap();

        // Fill with the background texture.  The assumption is that 4
        // background images are needed to cover the entire screen:
        // 
        // map
        // ┌──────────────────────────────────────────┐
        // │                  ┊                   ┊   │
        // │  cam             ┊                   ┊   │
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
        
        renderer.copy(self.background_texture, None, Some(top_left)).ok().unwrap();
        renderer.copy(self.background_texture, None, Some(top_right)).ok().unwrap();
        renderer.copy(self.background_texture, None, Some(bottom_left)).ok().unwrap();
        renderer.copy(self.background_texture, None, Some(bottom_right)).ok().unwrap();
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
        Vec2{x: f(p.x, self.w), y: f(p.y, self.h)}
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
        Vec2{x: f(p.x, w, self.w), y: f(p.y, h, self.h)}
    }
}

// ---------------------------------------------------------------------
// Main

#[deriving(PartialEq, Clone, Show, Copy)]
struct CameraSpec {
    acceleration: f64,
    // The minimum distance from the top/bottom edges to the ship
    v_padding: i32,
    // The minimum distance from the left/right edges to the ship
    h_padding: i32,
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct Camera {
    spec: CameraSpec,
    pos: Vec2<i32>,
}

impl Camera {
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
        let f = ship.speed * self.spec.acceleration;
        
        self.pos.x += (f.x * dt) as i32;
        self.pos.y += (f.y * dt) as i32;

        // Make sure the ship is not too much to the edge
        if self.left() + self.spec.h_padding > ship.pos.x {
            self.pos.x = ship.pos.x - self.spec.h_padding
        } else if self.right() - self.spec.h_padding < ship.pos.x {
            self.pos.x = (ship.pos.x + self.spec.h_padding) - SCREEN_WIDTH
        }
        if self.top() + self.spec.v_padding > ship.pos.y {
            self.pos.y = ship.pos.y - self.spec.v_padding
        } else if self.bottom() - self.spec.v_padding < ship.pos.y {
            self.pos.y = (ship.pos.y + self.spec.v_padding) - SCREEN_HEIGHT
        }

        // Make sure it stays in the map
        self.pos = map.bound_rect(self.pos, SCREEN_WIDTH, SCREEN_HEIGHT);
    }
}

#[deriving(PartialEq)]
struct State<'a> {
    quit: bool,
    accelerating: bool,
    rotating: Rotating,
    time_delta: f64,
    ship: Ship<'a>,
    map: Map<'a>,
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

fn run(renderer: &Renderer, state: &mut State, prev_time0: u32) {
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
        state.ship.render(renderer, state.accelerating, &state.camera);
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
        "Dogfights",
        sdl2::video::WindowPos::PosUndefined, sdl2::video::WindowPos::PosUndefined,
        (SCREEN_WIDTH as int), (SCREEN_HEIGHT as int),
        sdl2::video::SHOWN).ok().unwrap();
    let renderer = Renderer::from_window(
        window,
        sdl2::render::RenderDriverIndex::Auto,
        sdl2::render::ACCELERATED | sdl2::render::PRESENTVSYNC).ok().unwrap();
    renderer.set_logical_size((SCREEN_WIDTH as int), (SCREEN_HEIGHT as int)).ok().unwrap();
    let planes_surface = sdl2_image::LoadSurface::from_file(&from_str("assets/planes.png").unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture: &Texture = &renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let ship_pos = Vec2 {x: SCREEN_WIDTH / 2, y: SCREEN_HEIGHT / 2};
    let ship = Ship {
        spec : ShipSpec {
            color: Color::RGB(0x00, 0x00, 0x00),
            height: 30.,
            width: 20.,
            rotation_speed: 0.007,
            rotation_speed_accelerating: 0.002,
            acceleration: 0.035,
            friction: 0.02,
            gravity: 0.008,
            sprite: Sprite{
                texture: planes_texture,
                rect: Rect{x: 128, y: 96, w: 30, h: 24},
                center: Point{x: 15, y: 24},
                angle: 90.,
            },
            sprite_accelerating: Sprite{
                texture: planes_texture,
                rect: Rect{x: 88, y: 96, w: 30, h: 24},
                center: Point{x: 15, y: 24},
                angle: 90.,
            },
        },
        pos: ship_pos,
        speed: Vec2 {x: 0., y: 0.},
        rotation: 0.,
    };
    let map_surface = sdl2_image::LoadSurface::from_file(&from_str("assets/background.png").unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let map = Map {
        w: SCREEN_WIDTH*10,
        h: SCREEN_HEIGHT*10,
        background_color: Color::RGB(0x58, 0xB7, 0xFF),
        background_texture: &map_texture,
    };
    let mut state = State {
        quit: false,
        accelerating: false,
        rotating: Rotating::Still,
        time_delta: 0.,
        ship: ship,
        map: map,
        camera: Camera {
            spec: CameraSpec {
                acceleration: 1.2,
                h_padding: 220,
                v_padding: 220 * SCREEN_HEIGHT / SCREEN_WIDTH,
            },
            pos: Vec2{
                x: ship_pos.x - SCREEN_WIDTH/2,
                y: ship_pos.y - SCREEN_HEIGHT/2,
            }
        }
    };
    run(&renderer, &mut state, sdl2::get_ticks());
}
