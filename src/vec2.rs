use sdl2::rect::Point;

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
    pub fn point(self) -> Point {
        Point{x: self.x, y: self.y}
    }
}
