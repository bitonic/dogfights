extern crate sdl2;

use std::ops::{Add, Sub, Mul, Div, Neg};

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

    #[inline]
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {x : self.x - other.x, y: self.y - other.y}
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn mul(self, other: f32) -> Vec2 {
        Vec2 {x: self.x * other, y: self.y * other}
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;

    #[inline]
    fn div(self, other: f32) -> Vec2 {
        Vec2 {x: self.x / other, y: self.y / other}
    }
}

impl Neg for Vec2 {
    type Output = Vec2;

    #[inline]
    fn neg(self) -> Vec2 {
        Vec2{x: -self.x, y: -self.y}
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
