extern crate sdl2;

// ---------------------------------------------------------------------
// Input

// FIXME: efficient serialization using u8
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
    pub fn process_events(&mut self) {
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
                        sdl2::keycode::KeyCode::Up    => self.accel = true,
                        sdl2::keycode::KeyCode::X     => self.firing = true,
                        sdl2::keycode::KeyCode::P     => self.paused = !self.paused,
                        _                             => {},
                    },
                sdl2::event::Event::KeyUp(_, _, key, _, _, _) => {
                    if self.accel && key == sdl2::keycode::KeyCode::Up {
                        self.accel = false
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
