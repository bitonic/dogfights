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
// Utils

#[inline(always)]
fn to_radians(x: f64) -> f64 {
    x * std::f64::consts::PI/180.
}

#[inline(always)]
fn from_radians(x: f64) -> f64 {
    x * 180./std::f64::consts::PI
}


// ---------------------------------------------------------------------
// Sprites

#[deriving(PartialEq, Clone, Copy)]
struct Sprite<'tx> {
    texture: &'tx Texture,
    rect: Rect,
    center: Point,
    // If the sprite is already rotated by some angle
    angle: f64,
}

// impl<'tx> std::fmt::Show for Sprite<'tx> {
//     fn fmt(&self, fmter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match fmter.write_str("<<Sprite>>") {
//             Ok(()) => Ok(()),
//             Err(ioerr) =>
//         Ok(())
//     }
// }

impl<'tx> Sprite<'tx> {
    fn render(&self, renderer: &Renderer, rotation: f64, dst: Option<Rect>) -> SdlResult<()> {
        let dst = match dst {
            None       => None,
            Some(rect) => Some(Rect{x: rect.x - self.center.x, y: rect.y - self.center.y, .. rect}),
        };
        let angle = from_radians(rotation);
        renderer.copy_ex(
            self.texture, Some(self.rect), dst, self.angle - (-1. * angle),
            Some(self.center), sdl2::render::RendererFlip::None)
    }
}

// ---------------------------------------------------------------------
// Ship

#[deriving(PartialEq, Clone, Copy)]
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq, Clone, Copy)]
struct ShipSpec<'tx> {
    rotation_speed: f64,
    rotation_speed_accelerating: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
    sprite: Sprite<'tx>,
    sprite_accelerating: Sprite<'tx>,
    bullet_spec: BulletSpec<'tx>,
    firing_interval: u32,
}

#[deriving(PartialEq, Clone)]
struct Ship<'tx> {
    spec: ShipSpec<'tx>,
    pos: Vec2<i32>,
    speed: Vec2<f64>,
    rotation: f64,
    bullets: Vec<Bullet<'tx>>,
}

impl<'tx> Ship<'tx> {
    fn advance(&mut self, map: &Map, accelerating: bool, firing: bool, rotating: Rotating, dt: f64) -> () {
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

        // =============================================================
        // Advance the bullets
        self.bullets = Bullet::advance_bullets(&self.bullets, map, dt);

        // =============================================================
        // Add new bullet
        if firing {
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                pos: self.pos,
                rotation: self.rotation,
                age: 0.,
            };
            self.bullets.push(bullet);
        }
    }

    fn render(&self, renderer: &Renderer, accelerating: bool, cam: &Camera) -> () {
        // =============================================================
        // Render ship
        let pos = cam.adjust(self.pos);
        let dst = Rect{x: pos.x, y: pos.y, .. self.spec.sprite.rect};
        if accelerating {
            self.spec.sprite_accelerating.render(renderer, self.rotation, Some(dst)).ok().unwrap()
        } else {
            self.spec.sprite.render(renderer, self.rotation, Some(dst)).ok().unwrap()
        }

        // =============================================================
        // Render bullets
        for bullet in self.bullets.iter() {
            bullet.render(renderer, cam);
        }
    }
}

// ---------------------------------------------------------------------
// Bullets

#[deriving(PartialEq, Clone, Copy)]
struct BulletSpec<'tx> {
    sprite: Sprite<'tx>,
    speed: f64,
    lifetime: f64,
}

#[deriving(PartialEq, Clone, Copy)]
struct Bullet<'tx> {
    spec: BulletSpec<'tx>,
    pos: Vec2<i32>,
    rotation: f64,
    age: f64,
}

impl<'tx> Bullet<'tx> {
    fn advance(&self, dt: f64) -> Bullet<'tx> {
        let pos = Vec2 {
            x: self.pos.x + ((self.spec.speed * self.rotation.cos() * dt) as i32),
            y: self.pos.y + ((self.spec.speed * self.rotation.sin() * dt) as i32),
        };
        Bullet {pos: pos, rotation: self.rotation, age: self.age + dt, spec: self.spec}
    }

    fn advance_bullets(bullets: &Vec<Bullet<'tx>>, map: &Map, dt: f64) -> Vec<Bullet<'tx>> {
        let mut new_bullets = Vec::with_capacity(bullets.len() + 1);
        for bullet in bullets.iter() {
            let new_bullet = bullet.advance(dt);
            if new_bullet.alive(map) {
                new_bullets.push(new_bullet)
            }
        };
        new_bullets
    }

    fn alive(&self, map: &Map) -> bool {
        self.pos.x >= 0 && self.pos.x <= map.w &&
            self.pos.y >= 0 && self.pos.y <= map.h &&
            self.age < self.spec.lifetime
    }

    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        let pos = cam.adjust(self.pos);
        let dst = Rect{x: pos.x, y: pos.y, .. self.spec.sprite.rect};
        self.spec.sprite.render(renderer, self.rotation, Some(dst)).ok().unwrap()
    }
}

// ---------------------------------------------------------------------
// Shooter

struct ShooterSpec<'tx> {
    sprite: Sprite<'tx>,
    pos: Vec2<i32>,
    rotation: f64,
    bullet_spec: BulletSpec<'tx>,
    firing_rate: f64,
}

struct Shooter<'tx> {
    spec: ShooterSpec<'tx>,
    time_since_fire: f64,
    bullets: Vec<Bullet<'tx>>,
}

impl<'tx> Shooter<'tx> {
    fn advance(&mut self, map: &Map, dt: f64) {
        self.bullets = Bullet::advance_bullets(&self.bullets, map, dt);
        self.time_since_fire += dt;
        if self.time_since_fire > self.spec.firing_rate {
            self.time_since_fire = 0.;
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                pos: self.spec.pos,
                rotation: self.spec.rotation,
                age: 0.,
            };
            self.bullets.push(bullet);
        }            
    }

    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        let pos = cam.adjust(self.spec.pos);
        let dst = Rect{x: pos.x, y: pos.y, .. self.spec.sprite.rect};
        self.spec.sprite.render(renderer, self.spec.rotation, Some(dst)).ok().unwrap();
        for bullet in self.bullets.iter() {
            bullet.render(renderer, cam);
        }
    }
}

// ---------------------------------------------------------------------
// Maps

#[deriving(PartialEq, Clone, Copy)]
struct Map<'tx> {
    w: i32,
    h: i32,
    background_color: Color, 
    background_texture: &'tx Texture,
}

impl<'tx> Map<'tx> {
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

struct State<'tx> {
    quit: bool,
    accelerating: bool,
    last_fired: Option<u32>,
    firing: bool,
    rotating: Rotating,
    ship: Ship<'tx>,
    map: Map<'tx>,
    camera: Camera,
    shooters: Vec<Shooter<'tx>>,
}

impl<'tx> State<'tx> {
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
                        _                             => {},
                    },
                sdl2::event::Event::KeyUp(_, _, key, _, _, _) =>
                    match (self.accelerating, self.firing, self.rotating, key) {
                        (true, _, _, sdl2::keycode::KeyCode::Up) =>
                            self.accelerating = false,
                        (_, _, Rotating::Left, sdl2::keycode::KeyCode::Left) =>
                        self.rotating = Rotating::Still,
                        (_, _, Rotating::Right, sdl2::keycode::KeyCode::Right) =>
                            self.rotating = Rotating::Still,
                        (_, true, _, sdl2::keycode::KeyCode::X) =>
                            self.firing = false,
                        _ =>
                        {},
                    },
                _ =>
                {},
            }
        }
    }

    fn advance(&mut self, now: u32, dt: f64) {
        self.process_events();
        let firing = match (self.firing, self.last_fired) {
            (true, None) => {
                self.last_fired = Some(now);
                true
            },
            (true, Some(t)) =>
                if now - t > self.ship.spec.firing_interval {
                    self.last_fired = Some(now);
                    true
                } else {
                    false
                },
            _ =>
                false,
        };

        self.ship.advance(&self.map, self.accelerating, firing, self.rotating, dt);
        for i in range(0, self.shooters.len()) {
            self.shooters[i].advance(&self.map, dt);
        }
        self.camera.advance(&self.map, &self.ship, dt);
    }

    fn render(&self, renderer: &Renderer) {
        // Paint the background for the whole thing
        renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)).ok().unwrap();
        renderer.clear().ok().unwrap();
        // Paint the map
        self.map.render(renderer, &self.camera);
        // Paint the ship
        self.ship.render(renderer, self.accelerating, &self.camera);
        // Paint the shooters
        for shooter in self.shooters.iter() {
            shooter.render(renderer, &self.camera);
        }
        // GO
        renderer.present();
    }

    fn run(&mut self, renderer: &Renderer) {
        let mut prev_time = sdl2::get_ticks();
        loop {
            let time_now = sdl2::get_ticks();
            let dt = (time_now - prev_time) as f64;
            self.advance(time_now, dt);
            self.render(renderer);
            if self.quit {
                break;
            }
            prev_time = time_now;
        }
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
    let bullet_spec = BulletSpec {
        sprite: Sprite {
            texture: planes_texture,
            rect: Rect{x: 424, y: 140, w: 3, h: 12},
            center: Point{x: 1, y: 6},
            angle: 90.,
        },
        speed: 1.,
        lifetime: 5000.,
    };
    let ship_pos = Vec2 {x: SCREEN_WIDTH / 2, y: SCREEN_HEIGHT / 2};
    let ship = Ship {
        spec : ShipSpec {
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
            sprite_accelerating: Sprite {
                texture: planes_texture,
                rect: Rect{x: 88, y: 96, w: 30, h: 24},
                center: Point{x: 15, y: 24},
                angle: 90.,
            },
            bullet_spec: bullet_spec,
            firing_interval: 1000,
        },
        pos: ship_pos,
        speed: Vec2 {x: 0., y: 0.},
        rotation: 0.,
        bullets: Vec::new(),
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
        firing: false,
        last_fired: None,
        rotating: Rotating::Still,
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
        },
        shooters: vec![
            Shooter {
                spec: ShooterSpec {
                    sprite: Sprite {
                        texture: planes_texture,
                        rect: Rect{x: 48, y: 248, w: 32, h: 24},
                        center: Point{x: 16, y: 12},
                        angle: 90.,
                    },
                    pos: Vec2{x: 1000, y: 200},
                    rotation: to_radians(90.),
                    bullet_spec: bullet_spec,
                    firing_rate: 2000.,
                },
                time_since_fire: 0.,
                bullets: Vec::new(),
            }
            ],
    };
    state.run(&renderer);
}


/*

fn main() {
    println!("Ciao")
}
*/
