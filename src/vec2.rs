use sdl2::rect::Point;
use std::num::FloatMath;

#[deriving(PartialEq, Clone, Show, Copy)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T: Add<T, T>> Add<Vec2<T>, Vec2<T>> for Vec2<T> {
    fn add(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {x : self.x + other.x, y: self.y + other.y}
    }
}

impl<T: Sub<T, T>> Sub<Vec2<T>, Vec2<T>> for Vec2<T> {
    fn sub(self, other: Vec2<T>) -> Vec2<T> {
        Vec2 {x : self.x - other.x, y: self.y - other.y}
    }
}

impl<T: Copy + Mul<T, T>> Mul<T, Vec2<T>> for Vec2<T> {
    fn mul(self: Vec2<T>, other: T) -> Vec2<T> {
        Vec2 {x: self.x * other, y: self.y * other}
    }
}

impl Vec2<i32> {
    pub fn point(&self) -> Point {
        Point{x: self.x, y: self.y}
    }

    pub fn rotate_i32(self, center: Vec2<i32>, rotation: f64) -> Vec2<i32> {
        let v = Vec2{x: self.x as f64, y: self.y as f64};
        let center = Vec2{x: center.x as f64, y: center.y as f64};
        let v_ = v.rotate(center, rotation);
        Vec2{x: v_.x as i32, y: v_.y as i32}
    }
}

impl Vec2<f64> {
    pub fn rotate(self, center: Vec2<f64>, rotation: f64) -> Vec2<f64> {
        let dist_x = self.x - center.x;
        let dist_y = self.y - center.y;
        Vec2 {
            x: center.x + dist_x * rotation.cos() + dist_y * rotation.sin(),
            y: center.y - dist_x * rotation.sin() + dist_y * rotation.cos(),
        }
    }
}
