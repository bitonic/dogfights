extern crate sdl2;

use texture::TextureId;
use vec::Vec2;

// ---------------------------------------------------------------------
// Sprites

#[derive(Clone, Copy, RustcEncodable, RustcDecodable, PartialEq)]
pub struct Sprite {
    pub texture: TextureId,
    pub center: Vec2,
    // If the sprite is already rotated by some angle
    pub angle: f32,
}

// ---------------------------------------------------------------------
// Color (we don't use the SDL2 one because we can't encode/decode it)

#[derive(Clone, Copy, RustcEncodable, RustcDecodable, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub fn to_sdl_color(self) -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGB(self.0, self.1, self.2)
    }
}

// ---------------------------------------------------------------------
// Map

#[derive(PartialEq, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct Map {
    pub w: f32,
    pub h: f32,
    pub background_color: Color,
    pub background_texture: TextureId,
}

// ---------------------------------------------------------------------
// Spec

#[derive(PartialEq, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct CameraSpec {
    pub accel: f32,
    // The minimum distance from the top/bottom edges to the ship
    pub v_pad: f32,
    // The minimum distance from the left/right edges to the ship
    pub h_pad: f32,
}

#[derive(PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub struct ShipSpec {
    pub rotation_vel: f32,
    pub rotation_vel_accel: f32,
    pub accel: f32,
    pub friction: f32,
    pub gravity: f32,
    pub sprite: Sprite,
}

#[derive(PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub enum Spec {
    ShipSpec(ShipSpec)
}

#[derive(PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub struct GameSpec {
    pub map: Map,
    pub camera_spec: CameraSpec,
    pub ship_spec: ShipSpec,
}
