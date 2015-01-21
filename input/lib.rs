extern crate sdl2;
extern crate "rustc-serialize" as rustc_serialize;

// ---------------------------------------------------------------------
// Input

#[derive(PartialEq, Clone, Copy, Show, RustcDecodable, RustcEncodable)]
pub enum Rotating {
    Still,
    Left,
    Right,
}

#[derive(PartialEq, Clone, Copy, Show, RustcEncodable, RustcDecodable)]
pub struct Input {
    pub quit: bool,
    pub accel: bool,
    pub firing: bool,
    pub rotating: Rotating,
    pub paused: bool,
}

impl Input {
    pub fn new() -> Input {
        Input{
            quit: false,
            accel: false,
            firing: false,
            rotating: Rotating::Still,
            paused: false,
        }
    }

    pub fn process_events(self) -> Input {
        let mut input = self;
        loop {
            match sdl2::event::poll_event() {
                sdl2::event::Event::None =>
                    break,
                sdl2::event::Event::Quit(_) =>
                    input.quit = true,
                sdl2::event::Event::KeyDown(_, _, key, _, _, _) =>
                    match key {
                        sdl2::keycode::KeyCode::Left  => input.rotating = Rotating::Left,
                        sdl2::keycode::KeyCode::Right => input.rotating = Rotating::Right,
                        sdl2::keycode::KeyCode::Up    => input.accel = true,
                        sdl2::keycode::KeyCode::X     => input.firing = true,
                        sdl2::keycode::KeyCode::P     => input.paused = !input.paused,
                        _                             => {},
                    },
                sdl2::event::Event::KeyUp(_, _, key, _, _, _) => {
                    if input.accel && key == sdl2::keycode::KeyCode::Up {
                        input.accel = false
                    };
                    if input.firing && key == sdl2::keycode::KeyCode::X {
                        input.firing = false;
                    };
                    if input.rotating == Rotating::Left && key == sdl2::keycode::KeyCode::Left {
                        input.rotating = Rotating::Still;
                    };
                    if input.rotating == Rotating::Right && key == sdl2::keycode::KeyCode::Right {
                        input.rotating = Rotating::Still;
                    };
                },
                _ => {},
            }
        };
        input
    }
}
