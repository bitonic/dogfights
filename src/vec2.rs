use sdl2::rect::Point;
use std::num::FloatMath;

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
    pub fn point(&self) -> Point {
        Point{x: self.x as i32, y: self.y as i32}
    }


    pub fn rotate(self, center: Vec2, rotation: f64) -> Vec2 {
        let dist_x = self.x - center.x;
        let dist_y = self.y - center.y;
        Vec2 {
            x: center.x + dist_x * rotation.cos() + dist_y * rotation.sin(),
            y: center.y - dist_x * rotation.sin() + dist_y * rotation.cos(),
        }
    }
}
