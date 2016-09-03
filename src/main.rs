extern crate sdl2;
extern crate sdl2_image;
extern crate rustc_serialize;
extern crate bincode;

use std::collections::HashMap;
use std::path::Path;
use std::ops::{Add, Sub, Mul, Div};

// Textures
// --------------------------------------------------------------------

pub type TextureId = u32;
pub type Textures = HashMap<TextureId, sdl2::render::Texture>;

pub const PLANES_TEXTURE_ID: TextureId = 0;
pub const MAP_TEXTURE_ID: TextureId = 1;

// ---------------------------------------------------------------------
// Vec

#[derive(Clone, Copy, RustcEncodable, RustcDecodable, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {x : self.x + other.x, y: self.y + other.y}
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {x : self.x - other.x, y: self.y - other.y}
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self: Vec2, other: f32) -> Vec2 {
        Vec2 {x: self.x * other, y: self.y * other}
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self: Vec2, other: f32) -> Vec2 {
        Vec2 {x: self.x / other, y: self.y / other}
    }
}

impl Vec2 {
    #[inline]
    pub fn point(self) -> sdl2::rect::Point {
        sdl2::rect::Point::new(self.x as i32, self.y as i32)
    }

    // pub fn rotate_centered(&self, center: &Vec2, rotation: f32) -> Vec2 {
    //     let x_diff = self.x - center.x;
    //     let y_diff = self.y - center.y;
    //     Vec2 {
    //         x: center.x + x_diff * rotation.cos() + y_diff * rotation.sin(),
    //         y: center.y - x_diff * rotation.sin() + y_diff * rotation.cos(),
    //     }
    // }

    // We rotate clockwise because SDL does so too -- the y axes starts
    // from 0 at the top and decreases going down.
    #[inline]
    pub fn rotate(self, rotation: f32) -> Vec2 {
        Vec2 {
            x: self.x * rotation.cos() + self.y * rotation.sin(),
            y: self.y * rotation.cos() - self.x * rotation.sin(),
        }
    }

    #[inline]
    pub fn mag(self) -> f32 {
        (self.x*self.x + self.y*self.y).sqrt()
    }

    #[inline]
    pub fn zero() -> Vec2 {
        Vec2{x: 0., y: 0.}
    }

    #[inline]
    pub fn norm(self) -> Vec2 {
        self / self.mag()
    }
}

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

// Init
// --------------------------------------------------------------------

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

fn init_sdl(vsync: bool) -> sdl2::render::Renderer<'static> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Dogfights", SCREEN_WIDTH, SCREEN_HEIGHT)
            .position_centered()
            .opengl()
            .allow_highdpi()
            .build()
            .unwrap();
    let renderer_builder0 = window.renderer();
    let renderer_builder = if vsync { renderer_builder0.present_vsync() } else { renderer_builder0 };
    renderer_builder.accelerated().build().unwrap()
}

fn init_textures<'a>(renderer: &sdl2::render::Renderer<'a>) -> Textures {
    let mut textures = HashMap::new();

    let mut planes_surface: sdl2::surface::Surface =
            sdl2_image::LoadSurface::from_file(Path::new("assets/planes.png")).ok().unwrap();
    planes_surface.set_color_key(true, sdl2::pixels::Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture = renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let _ = textures.insert(PLANES_TEXTURE_ID, planes_texture);

    let map_surface: sdl2::surface::Surface = sdl2_image::LoadSurface::from_file(Path::new("assets/background.png")).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let _ = textures.insert(MAP_TEXTURE_ID, map_texture);

    textures
}

fn main() {
    let renderer = init_sdl(true);
    let _textures = init_textures(&renderer);
    println!("Hello, world!");
}
