extern crate sdl2;
extern crate sdl2_image;

use sdl2::pixels::Color;
use sdl2::SdlResult;
use sdl2::render::{Renderer, Texture};
use std::num::FloatMath;

use geometry::{to_radians, from_radians, Vec2, Rect, Transform, overlapping};
use physics::Interpolate;

pub mod geometry;
pub mod physics;

// ---------------------------------------------------------------------
// Constants

static SCREEN_WIDTH: f64 = 800.;
static SCREEN_HEIGHT: f64 = 600.;
// 10 ms timesteps
static TIME_STEP: f64 = 0.01;
static MAX_FRAME_TIME: f64 = 0.250;

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
    fn render(&self, renderer: &Renderer, cam: &Camera, trans: &Transform) {
        // renderer.set_draw_color(Color::RGB(0xFF, 0x00, 0x00)).ok().unwrap();
        // for rect in self.rects.iter() {
        //     let trans = cam.adjust(trans);
        //     let (tl, tr, bl, br) = rect.transform(&trans);
        //     renderer.draw_line(tl.point(), tr.point()).ok().unwrap();
        //     renderer.draw_line(tr.point(), br.point()).ok().unwrap();
        //     renderer.draw_line(br.point(), bl.point()).ok().unwrap();
        //     renderer.draw_line(bl.point(), tl.point()).ok().unwrap();
        // }
    }
}

fn overlapping_bbox(this: &BBox, this_t: &Transform, other: &BBox, other_t: &Transform) -> bool {
    let mut overlap = false;
    for this in this.rects.iter() {
        if overlap { break };
        for other in other.rects.iter() {
            if overlap { break };
            overlap = overlapping(this, this_t, other, other_t);
        }
    }
    overlap
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

impl<'a> physics::Interpolate for Camera<'a> {
    #[inline]
    fn interpolate(&self, next: &Camera<'a>, alpha: f64) -> Camera<'a> {
        let previous = physics::State{pos: self.pos, v: self.velocity};
        let current = physics::State{pos: next.pos, v: next.velocity};
        let middle = previous.interpolate(&current, alpha);
        Camera{pos: middle.pos, velocity: middle.v, .. *self}
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
    rotation_velocity: f64,
    rotation_velocity_accelerating: f64,
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
    velocity: Vec2,
    bullets: Vec<Bullet<'a>>,
    not_firing_for: f64,
    accelerating: bool,
    rotating: Rotating,
}

impl<'a> physics::Acceleration for Ship<'a> {
    fn acceleration(&self, state: &physics::State) -> Vec2 {
        let mut f = Vec2::zero();
        // Acceleration
        if self.accelerating {
            f.x += self.trans.rotation.cos() * self.spec.acceleration;
            // The sin is inverted because we push the opposite
            // direction we're looking at.
            f.y += -1. * self.trans.rotation.sin() * self.spec.acceleration;
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

impl<'a> physics::Interpolate for Ship<'a> {
    fn interpolate(&self, next: &Ship<'a>, alpha: f64) -> Ship<'a> {
        let st = self.phys_state().interpolate(&next.phys_state(), alpha);
        if alpha < 0.5 {
            self.set_phys_state(&st)
        } else {
            next.set_phys_state(&st)
        }
    }
}

impl<'a> Ship<'a> {
    #[inline]
    fn phys_state(&self) -> physics::State {
        physics::State {
            pos: self.trans.pos,
            v: self.velocity,

        }
    }

    #[inline]
    fn set_phys_state(&self, state: &physics::State) -> Ship<'a> {
        Ship {
            trans: Transform{pos: state.pos, rotation: self.trans.rotation},
            velocity: state.v,
            .. self.clone()
        }
    }

    fn advance(&self, map: &Map, input: &Input, hits: uint, dt: f64) -> Ship<'a> {
        let accelerating = input.accelerating;
        let rotating = input.rotating;
        let mut not_firing_for = self.not_firing_for + dt;
        let firing = if input.firing && self.not_firing_for >= self.spec.firing_interval {
            not_firing_for = 0.;
            true
        } else {
            false
        };
        let mut trans = self.trans;
        let mut velocity = self.velocity;

        // =============================================================
        // Apply the rotation
        let rotation_velocity = if accelerating {
            self.spec.rotation_velocity_accelerating
        } else {
            self.spec.rotation_velocity
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
        let st = physics::integrate(self, &st, dt);
        velocity = st.v;
        trans.pos = st.pos;
        trans.pos = map.bound(trans.pos);

        // =============================================================
        // Advance the bullets
        let mut bullets = Bullet::advance_bullets(&self.bullets, map, dt);

        // =============================================================
        // Add new bullet
        if firing {
            let shoot_from = self.spec.shoot_from.rotate(trans.rotation);
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                trans: trans + shoot_from,
                age: 0.,
            };
            bullets.push(bullet);
        }

        // =============================================================
        // Decrease health if hit
        if hits > 0 {
            println!("Hits: {}", hits);
        }
        
        Ship {
            spec: self.spec,
            trans: trans,
            velocity: velocity,
            bullets: bullets,
            not_firing_for: not_firing_for,
            accelerating: accelerating,
            rotating: rotating,
        }
    }

    fn render(&self, renderer: &Renderer, cam: &Camera) {
        // =============================================================
        // Render ship
        let trans = cam.adjust(&self.trans);
        if self.accelerating {
            self.spec.sprite_accelerating.render(renderer, &trans).ok().unwrap()
        } else {
            self.spec.sprite.render(renderer, &trans).ok().unwrap()
        }

        // =============================================================
        // Debugging -- render bbox
        self.spec.bbox.render(renderer, cam, &self.trans);

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
    velocity: f64,
    lifetime: f64,
    bbox: &'a BBox,
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
            x: self.trans.pos.x + (self.spec.velocity * self.trans.rotation.cos() * dt),
            y: self.trans.pos.y + (-1. * self.spec.velocity * self.trans.rotation.sin() * dt),
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
        self.spec.sprite.render(renderer, &cam.adjust(&self.trans)).ok().unwrap();
        // Debugging -- render bbox
        self.spec.bbox.render(renderer, cam, &self.trans);
    }
}

// ---------------------------------------------------------------------
// Shooter

#[deriving(PartialEq, Clone, Copy)]
struct ShooterSpec<'a> {
    sprite: &'a Sprite<'a>,
    trans: Transform,
    bullet_spec: &'a BulletSpec<'a>,
    firing_rate: f64,
}

#[deriving(PartialEq, Clone)]
struct Shooter<'a> {
    spec: &'a ShooterSpec<'a>,
    time_since_fire: f64,
    bullets: Vec<Bullet<'a>>,
}

impl<'a> Shooter<'a> {
    fn advance(&self, map: &Map, dt: f64) -> Shooter<'a> {
        let mut bullets = Bullet::advance_bullets(&self.bullets, map, dt);
        let mut time_since_fire = self.time_since_fire + dt;
        if time_since_fire > self.spec.firing_rate {
            time_since_fire = 0.;
            let bullet = Bullet {
                spec: self.spec.bullet_spec,
                trans: self.spec.trans,
                age: 0.,
            };
            bullets.push(bullet);
        }
        Shooter{spec: self.spec, time_since_fire: time_since_fire, bullets: bullets}
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

#[deriving(PartialEq, Clone)]
struct State<'a> {
    map: &'a Map<'a>,
    ship: Ship<'a>,
    camera: Camera<'a>,
    shooters: Vec<Shooter<'a>>,
}

impl<'a> physics::Interpolate for State<'a> {
    fn interpolate(&self, next: &State<'a>, alpha: f64) -> State<'a> {
        State{
            map: self.map,
            ship: self.ship.interpolate(&next.ship, alpha),
            camera: self.camera.interpolate(&next.camera, alpha),
            shooters: if alpha < 0.5 { self.shooters.clone() } else { next.shooters.clone() },
        }
    }
}

impl<'a> State<'a> {
    fn advance(&self, input: &Input, dt: f64) -> State<'a> {
        if !input.paused {
            // Calculate hits
            let mut hits = 0;
            for i in range(0, self.shooters.len()) {
                for bullet in self.shooters[i].bullets.iter() {
                    if overlapping_bbox(self.ship.spec.bbox, &self.ship.trans, bullet.spec.bbox, &bullet.trans) {
                        hits += 1;
                    }
                }
            }
            // Advance stuff
            let ship = self.ship.advance(self.map, input, hits, dt);
            let mut shooters = Vec::with_capacity(self.shooters.len());
            for i in range(0, self.shooters.len()) {
                shooters.push(self.shooters[i].advance(self.map, dt));
            }
            let camera = self.camera.advance(self.map, &ship, dt);
            State{map: self.map, ship: ship, camera: camera, shooters: shooters}
        } else {
            self.clone()
        }
    }

    fn render(&self, renderer: &Renderer) {
        // Paint the background for the whole thing
        renderer.set_draw_color(Color::RGB(0x00, 0x00, 0x00)).ok().unwrap();
        renderer.clear().ok().unwrap();
        // Paint the map
        self.map.render(renderer, &self.camera);
        // Paint the ship
        self.ship.render(renderer, &self.camera);
        // Paint the shooters
        for shooter in self.shooters.iter() {
            shooter.render(renderer, &self.camera);
        }
        // GO
        renderer.present();
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
                // state = previous.interpolate(&current, accumulator/TIME_STEP);
                state = current;
            }

            state.render(renderer);
        }
    }
}

fn main() {
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
        velocity: 1000.,
        lifetime: 5000.,
        bbox: &BBox {
            rects: vec![
                Rect{
                    pos: Vec2{y: -1.5, x: -6.},
                    h: 3.,
                    w: 12.
                }]
        },
    };
    let ship_pos = Vec2 {x: SCREEN_WIDTH/2., y: SCREEN_HEIGHT/2.};
    let ship = Ship {
        spec: &ShipSpec {
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
            bullet_spec: bullet_spec,
            firing_interval: 1.,
            shoot_from: Vec2{x: 18., y: 0.},
            bbox: &BBox{
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
        },
        trans: Transform {
            pos: ship_pos,
            rotation: 0.,
        },
        velocity: Vec2 {x: 0., y: 0.},
        bullets: Vec::new(),
        not_firing_for: 100000.,
        accelerating: false,
        rotating: Rotating::Still,
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
    };
    let state = State {
        ship: ship,
        map: map,
        camera: Camera {
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

// fn main() {
//     println!("Ciao")
// }

