extern crate sdl2;
extern crate sdl2_image;
extern crate "rustc-serialize" as rustc_serialize;
extern crate bincode;

pub mod geometry;
pub mod physics;
pub mod game;
pub mod network;

// fn main() { game::client() }
fn main() { game::server(&"127.0.0.1:10000") }
// fn main() {}
