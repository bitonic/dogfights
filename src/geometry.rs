extern crate sdl2;

use std::num::FloatMath;

// ---------------------------------------------------------------------
// Transform

#[deriving(PartialEq, Clone, Copy)]
pub struct Transform {
    pub pos: Vec2,
    pub rotation: f64,
}

impl Add<Vec2, Transform> for Transform {
    fn add(self, other: Vec2) -> Transform {
        Transform {
            pos: self.pos + other,
            rotation: self.rotation
        }
    }
}

impl Sub<Vec2, Transform> for Transform {
    fn sub(self, other: Vec2) -> Transform {
        Transform {
            pos: self.pos - other,
            rotation: self.rotation
        }
    }
}

// ---------------------------------------------------------------------
// Vec

#[deriving(PartialEq, Clone, Show, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Add<Vec2, Vec2> for Vec2 {
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {x : self.x + other.x, y: self.y + other.y}
    }
}

impl Sub<Vec2, Vec2> for Vec2 {
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {x : self.x - other.x, y: self.y - other.y}
    }
}

impl Mul<f64, Vec2> for Vec2 {
    fn mul(self: Vec2, other: f64) -> Vec2 {
        Vec2 {x: self.x * other, y: self.y * other}
    }
}

impl Vec2 {
    pub fn point(&self) -> sdl2::rect::Point {
        sdl2::rect::Point{x: self.x as i32, y: self.y as i32}
    }


    pub fn rotate(&self, rotation: f64) -> Vec2 {
        Vec2 {
            x:      self.x * rotation.cos() + self.y * rotation.sin(),
            y: 0. - self.y * rotation.sin() + self.y * rotation.cos(),
        }
    }

    pub fn transform(&self, trans: &Transform) -> Vec2 {
        self.rotate(trans.rotation) + trans.pos
    }
}

// ---------------------------------------------------------------------
// Rect

#[deriving(PartialEq, Clone, Show, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn sdl_rect(&self) -> sdl2::rect::Rect {
        sdl2::rect::Rect {
            x: self.x as i32,
            y: self.y as i32,
            w: self.w as i32,
            h: self.h as i32,
        }
    }
}
