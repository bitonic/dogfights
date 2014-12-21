extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::num::FloatMath;
use std::num::SignedInt;

static SCREEN_WIDTH: i32 = 1024;
static SCREEN_HEIGHT: i32 = 768;

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
enum Rotating {
    Still,
    Left,
    Right,
}

#[deriving(PartialEq, Clone, Show, Copy)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

impl Add<Vec2, Vec2> for Vec2 {
    fn add(self: Vec2, other: Vec2) -> Vec2 {
        Vec2 {x : self.x + other.x, y: self.y + other.y}
    }
}

impl Add<Vec2, Point> for Point {
    fn add(self: Point, other: Vec2) -> Point {
        Point {x: self.x + (other.x as i32), y: self.y + (other.y as i32)}
    }
}

impl Sub<Vec2, Vec2> for Vec2 {
    fn sub(self: Vec2, other: Vec2) -> Vec2 {
        Vec2 {x : self.x - other.x, y: self.y - other.y}
    }
}

impl Mul<f64, Vec2> for Vec2 {
    fn mul(self: Vec2, other: f64) -> Vec2 {
        Vec2 {x: self.x * other, y: self.y * other}
    }
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct Ship {
    spec: ShipSpec,
    pos: Point,
    speed: Vec2,
    rotation: f64,
}

impl Ship {
    fn advance(&mut self, accelerating: bool, rotating: Rotating, dt: u32) -> () {
        let dt = dt as f64;

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
        self.pos = self.pos + self.speed;
        if self.pos.x < 0 {
            self.pos.x = SCREEN_WIDTH - (SCREEN_WIDTH % self.pos.x.abs());
        } else {
            self.pos.x = self.pos.x % SCREEN_WIDTH;
        }
        if self.pos.y < 0 {
            self.pos.y = SCREEN_HEIGHT - (SCREEN_HEIGHT % self.pos.y.abs());
        } else {
            self.pos.y = self.pos.y % SCREEN_HEIGHT;
        }
    }

    fn render(&self, renderer: &sdl2::render::Renderer) -> () {
        renderer.set_draw_color(self.spec.color).ok().expect("Could not set color");

        let xf = self.pos.x as f64;
        let yf = self.pos.y as f64;
        let tip = Point {
            x: (xf + self.rotation.cos() * self.spec.height) as i32,
            y: (yf + self.rotation.sin() * self.spec.height) as i32,
        };
        let ortogonal = self.rotation + 1.57079633;
        let base_l = Point {
            x: (xf + ortogonal.cos() * self.spec.width/2.) as i32,
            y: (yf + ortogonal.sin() * self.spec.width/2.) as i32,
        };
        let base_r = Point {
            x: (xf - ortogonal.cos() * self.spec.width/2.) as i32,
            y: (yf - ortogonal.sin() * self.spec.width/2.) as i32,
        };

        renderer.draw_line(tip, base_l).ok().expect("Could not draw line");
        renderer.draw_line(tip, base_r).ok().expect("Could not draw line");
        renderer.draw_line(base_l, base_r).ok().expect("Could not draw line");
    }
}

#[deriving(PartialEq, Clone, Show, Copy)]
struct State {
    quit: bool,
    accelerating: bool,
    rotating: Rotating,
    time_delta: u32,
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

fn run(renderer: &sdl2::render::Renderer, ship: &mut Ship, state: &mut State, prev_time0: u32) {
    let mut prev_time = prev_time0;
    loop {
        let time_now = sdl2::get_ticks();
        state.time_delta = time_now - prev_time;

        renderer.set_draw_color(Color::RGB(0xFF, 0xFF, 0xFF)).ok().expect("Could not set color");
        renderer.clear().ok().expect("Could not clear screen");

        process_events(state);
        ship.advance(state.accelerating, state.rotating, state.time_delta);
        ship.render(renderer);

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
        sdl2::video::SHOWN).ok().expect("Could not create window");
    let renderer = sdl2::render::Renderer::from_window(
        window,
        sdl2::render::RenderDriverIndex::Auto,
        sdl2::render::ACCELERATED | sdl2::render::PRESENTVSYNC).ok().expect("Could not create renderer");
    let mut ship = Ship {
        spec : ShipSpec {
            color: Color::RGB(0x00, 0x00, 0x00),
            height: 50.,
            width: 40.,
            rotation_speed: 0.005,
            acceleration: 0.025,
            friction: 0.001,
            gravity: 0.008,
        },
        pos: Point {x: SCREEN_WIDTH / 2, y: SCREEN_HEIGHT / 2},
        speed: Vec2 {x: 0., y: 0.},
        rotation: 0.,
    };
    let mut state = State {
        quit: false,
        accelerating: false,
        rotating: Rotating::Still,
        time_delta: 0,
    };
    run(&renderer, &mut ship, &mut state, sdl2::get_ticks());
}
