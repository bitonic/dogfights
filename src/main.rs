extern crate sdl2;
extern crate sdl2_image;

use sdl2::pixels::Color;
use sdl2::SdlResult;
use sdl2::render::{Renderer, Texture};
use std::num::FloatMath;
use geometry::{to_radians, from_radians, Vec2, Rect, Transform};

pub mod geometry;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: f64 = 800.;
static SCREEN_HEIGHT: f64 = 600.;

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

// impl<'a> std::fmt::Show for Sprite<'a> {
//     fn fmt(&self, fmter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match fmter.write_str("<<Sprite>>") {
//             Ok(()) => Ok(()),
//             Err(ioerr) =>
//         Ok(())
//     }
// }

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
// Bounding boxes

#[deriving(PartialEq, Clone)]
struct BBox {
    rects: Vec<Rect>,
}

impl BBox {
    fn overlaps(&self, self_t: &Transform, other: &BBox, other_t: &Transform) -> bool {
        let mut overlap = false;
        for this in self.rects.iter() {
            if overlap { break };
            for other in other.rects.iter() {
                if overlap { break };
                overlap = this.overlaps(self_t, other, other_t);
            }
        }
        overlap
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
struct ShipSpec<'a> {
    rotation_speed: f64,
    rotation_speed_accelerating: f64,
    acceleration: f64,
    friction: f64,
    gravity: f64,
    sprite: &'a Sprite<'a>,
    sprite_accelerating: &'a Sprite<'a>,
    bullet_spec: &'a BulletSpec<'a>,
    firing_interval: f64,
    shoot_from: Vec2,
    bbox: &'a BBox,
}

#[deriving(PartialEq, Clone)]
struct Ship<'a> {
    spec: &'a ShipSpec<'a>,
    trans: Transform,
    speed: Vec2,
    bullets: Vec<Bullet<'a>>,
    not_firing_for: f64,
}

impl<'a> Ship<'a> {
    fn advance(&mut self, map: &Map, input: &Input, dt: f64) -> () {
        self.not_firing_for += dt;
        let firing = if input.firing && self.not_firing_for >= self.spec.firing_interval {
            self.not_firing_for = 0.;
            true
        } else {
            false
        };

        // =============================================================
        // Apply the rotation
        let rotation_speed = if input.accelerating {
            self.spec.rotation_speed_accelerating
        } else {
            self.spec.rotation_speed
        };
        let rotation_delta = dt * rotation_speed;
        match input.rotating {
            Rotating::Still => {},
            Rotating::Left  => self.trans.rotation += rotation_delta,
            Rotating::Right => self.trans.rotation -= rotation_delta,
        }

        // =============================================================
        // Apply the force
        let mut f = Vec2 {x : 0., y: 0.};
        // Acceleration
        if input.accelerating {
            f.x += self.trans.rotation.cos() * self.spec.acceleration;
            // The sin is inverted because we push the opposite
            // direction we're looking at.
            f.y += -1. * self.trans.rotation.sin() * self.spec.acceleration;
        }

        // Gravity
        f.y += self.spec.gravity;

        // Friction
        let friction = self.speed * self.spec.friction;
        f = f - friction;

        // Update speed
        self.speed = self.speed + f;

        // Update position
        self.trans.pos = self.trans.pos + self.speed * dt;
        self.trans.pos = map.bound(self.trans.pos);

        // =============================================================
        // Advance the bullets
        self.bullets = Bullet::advance_bullets(&self.bullets, map, dt);

        // =============================================================
        // Add new bullet
        if firing {
            let shoot_from = self.spec.shoot_from.rotate(self.trans.rotation);
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                trans: self.trans + shoot_from,
                age: 0.,
            };
            self.bullets.push(bullet);
        }
    }

    fn render(&self, renderer: &Renderer, input: &Input, cam: &Camera) -> () {
        // =============================================================
        // Render ship
        let trans = cam.adjust(&self.trans);
        if input.accelerating {
            self.spec.sprite_accelerating.render(renderer, &trans).ok().unwrap()
        } else {
            self.spec.sprite.render(renderer, &trans).ok().unwrap()
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
struct BulletSpec<'a> {
    sprite: &'a Sprite<'a>,
    speed: f64,
    lifetime: f64,
}

#[deriving(PartialEq, Clone, Copy)]
struct Bullet<'a> {
    spec: &'a BulletSpec<'a>,
    trans: Transform,
    age: f64,
}

impl<'a> Bullet<'a> {
    fn advance(&self, dt: f64) -> Bullet<'a> {
        let pos = Vec2 {
            x: self.trans.pos.x + (self.spec.speed * self.trans.rotation.cos() * dt),
            y: self.trans.pos.y + (-1. * self.spec.speed * self.trans.rotation.sin() * dt),
        };
        Bullet {
            trans: Transform{pos: pos, rotation: self.trans.rotation},
            age: self.age + dt,
            spec: self.spec,
        }
    }

    fn advance_bullets(bullets: &Vec<Bullet<'a>>, map: &Map, dt: f64) -> Vec<Bullet<'a>> {
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
        self.trans.pos.x >= 0. && self.trans.pos.x <= map.w &&
            self.trans.pos.y >= 0. && self.trans.pos.y <= map.h &&
            self.age < self.spec.lifetime
    }

    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        self.spec.sprite.render(renderer, &cam.adjust(&self.trans)).ok().unwrap()
    }
}

// ---------------------------------------------------------------------
// Shooter

struct ShooterSpec<'a> {
    sprite: &'a Sprite<'a>,
    trans: Transform,
    bullet_spec: &'a BulletSpec<'a>,
    firing_rate: f64,
    bbox: &'a BBox,
}

struct Shooter<'a> {
    spec: &'a ShooterSpec<'a>,
    time_since_fire: f64,
    bullets: Vec<Bullet<'a>>,
}

impl<'a> Shooter<'a> {
    fn advance(&mut self, map: &Map, dt: f64) {
        self.bullets = Bullet::advance_bullets(&self.bullets, map, dt);
        self.time_since_fire += dt;
        if self.time_since_fire > self.spec.firing_rate {
            self.time_since_fire = 0.;
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                trans: self.spec.trans,
                age: 0.,
            };
            self.bullets.push(bullet);
        }            
    }

    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        self.spec.sprite.render(renderer, &cam.adjust(&self.spec.trans)).ok().unwrap();
        for bullet in self.bullets.iter() {
            bullet.render(renderer, cam);
        }
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
    fn render(&self, renderer: &Renderer, cam: &Camera) -> () {
        // Fill the whole screen with the background color
        renderer.set_draw_color(self.background_color).ok().unwrap();
        let rect = sdl2::rect::Rect {
            x: 0, y: 0, w: SCREEN_WIDTH as i32, h: SCREEN_HEIGHT as i32
        };
        renderer.fill_rect(&rect).ok().unwrap();

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
        let bgr_w = bgr.width as f64;
        let bgr_h = bgr.height as f64;
        let t = Vec2 {
            x: bgr_w - (cam.pos.x % bgr_w),
            y: bgr_h - (cam.pos.y % bgr_h),
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
        
        renderer.copy(self.background_texture, None, to_rect(top_left)).ok().unwrap();
        renderer.copy(self.background_texture, None, to_rect(top_right)).ok().unwrap();
        renderer.copy(self.background_texture, None, to_rect(bottom_left)).ok().unwrap();
        renderer.copy(self.background_texture, None, to_rect(bottom_right)).ok().unwrap();
    }

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
}

// ---------------------------------------------------------------------
// Main

#[deriving(PartialEq, Clone, Show, Copy)]
struct CameraSpec {
    acceleration: f64,
    // The minimum distance from the top/bottom edges to the ship
    v_padding: f64,
    // The minimum distance from the left/right edges to the ship
    h_padding: f64,
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct Camera {
    spec: CameraSpec,
    pos: Vec2,
}

impl Camera {
    fn adjust(&self, trans: &Transform) -> Transform {
        *trans - self.pos
    }

    #[inline(always)]
    fn left(&self) -> f64 { self.pos.x }
    #[inline(always)]
    fn right(&self) -> f64 { self.pos.x + SCREEN_WIDTH }
    #[inline(always)]
    fn top(&self) -> f64 { self.pos.y }
    #[inline(always)]
    fn bottom(&self) -> f64 { self.pos.y + SCREEN_HEIGHT }

    fn advance(&mut self, map: &Map, ship: &Ship, dt: f64) {
        // Push the camera based on the ship velocity
        let f = ship.speed * self.spec.acceleration;
        
        self.pos = self.pos + f * dt;

        // Make sure the ship is not too much to the edge
        if self.left() + self.spec.h_padding > ship.trans.pos.x {
            self.pos.x = ship.trans.pos.x - self.spec.h_padding
        } else if self.right() - self.spec.h_padding < ship.trans.pos.x {
            self.pos.x = (ship.trans.pos.x + self.spec.h_padding) - SCREEN_WIDTH
        }
        if self.top() + self.spec.v_padding > ship.trans.pos.y {
            self.pos.y = ship.trans.pos.y - self.spec.v_padding
        } else if self.bottom() - self.spec.v_padding < ship.trans.pos.y {
            self.pos.y = (ship.trans.pos.y + self.spec.v_padding) - SCREEN_HEIGHT
        }

        // Make sure it stays in the map
        self.pos = map.bound_rect(self.pos, SCREEN_WIDTH, SCREEN_HEIGHT);
    }
}

struct Input {
    quit: bool,
    accelerating: bool,
    firing: bool,
    rotating: Rotating
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

}

struct State<'a> {
    input: Input,
    ship: Ship<'a>,
    map: &'a Map<'a>,
    camera: Camera,
    shooters: Vec<Shooter<'a>>,
}

impl<'a> State<'a> {
    fn advance(&mut self, dt: f64) {
        self.input.process_events();
        self.ship.advance(self.map, &self.input, dt);
        for i in range(0, self.shooters.len()) {
            self.shooters[i].advance(self.map, dt);
        }
        self.camera.advance(self.map, &self.ship, dt);
    }

    fn render(&self, renderer: &Renderer) {
        // Paint the background for the whole thing
        renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)).ok().unwrap();
        renderer.clear().ok().unwrap();
        // Paint the map
        self.map.render(renderer, &self.camera);
        // Paint the ship
        self.ship.render(renderer, &self.input, &self.camera);
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
            self.advance(dt);
            self.render(renderer);
            if self.input.quit {
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
    let planes_surface = sdl2_image::LoadSurface::from_file(&("assets/planes.png".parse()).unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture: &Texture = &renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let bullet_spec = &BulletSpec {
        sprite: &Sprite {
            texture: planes_texture,
            rect: Rect{pos: Vec2{x: 424., y: 140.}, w: 3., h: 12.},
            center: Vec2{x: 1., y: 6.},
            angle: 90.,
        },
        speed: 1.,
        lifetime: 5000.,
    };
    let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
    let ship = Ship {
        spec: &ShipSpec {
            rotation_speed: 0.01,
            rotation_speed_accelerating: 0.001,
            acceleration: 0.025,
            friction: 0.02,
            gravity: 0.008,
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
            bullet_spec: bullet_spec,
            firing_interval: 1000.,
            shoot_from: Vec2{x: 18., y: 0.},
            bbox: &BBox{rects: vec![Rect{
                pos: Vec2{x: -15., y: -12.},
                w: 30.,
                h: 30.
            }]},
        },
        trans: Transform {
            pos: ship_pos,
            rotation: 0.,
        },
        speed: Vec2 {x: 0., y: 0.},
        bullets: Vec::new(),
        not_firing_for: 100000.,
    };
    let map_surface = sdl2_image::LoadSurface::from_file(&("assets/background.png".parse()).unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let map = &Map {
        w: SCREEN_WIDTH*10.,
        h: SCREEN_HEIGHT*10.,
        background_color: Color::RGB(0x58, 0xB7, 0xFF),
        background_texture: &map_texture,
    };
    let shooter_spec = &ShooterSpec {
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
        bullet_spec: bullet_spec,
        firing_rate: 2000.,
        bbox: &BBox{rects: vec![Rect{
            pos: Vec2{x: -16., y: -12.},
            w: 32.,
            h: 24.
        }]},
    };
    let mut state = State {
        input: Input{
            quit: false,
            accelerating: false,
            firing: false,
            rotating: Rotating::Still,
        },
        ship: ship,
        map: map,
        camera: Camera {
            spec: CameraSpec {
                acceleration: 1.2,
                h_padding: 220.,
                v_padding: 220. * SCREEN_HEIGHT / SCREEN_WIDTH,
            },
            pos: Vec2{
                x: ship_pos.x - SCREEN_WIDTH/2.,
                y: ship_pos.y - SCREEN_HEIGHT/2.,
            }
        },
        shooters: vec![
            Shooter {
                spec: shooter_spec,
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
