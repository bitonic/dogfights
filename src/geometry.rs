extern crate sdl2;

use std::num::FloatMath;
use std::num::Float;
use std::f64::consts::PI;

// ---------------------------------------------------------------------
// Angles

#[inline(always)]
pub fn to_radians(x: f64) -> f64 {
    x * PI/180.
}

#[inline(always)]
pub fn from_radians(x: f64) -> f64 {
    x * 180./PI
}

// ---------------------------------------------------------------------
// Transform

#[deriving(PartialEq, Clone, Copy, Show)]
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

impl Transform {
    pub fn id() -> Transform {
        Transform{pos: Vec2{x: 0., y: 0.}, rotation: 0.}
    }

    pub fn pos(pos: Vec2) -> Transform {
        Transform{pos: pos, rotation: 0.}
    }

    pub fn adjust(&self, other: &Transform) -> Transform {
        Transform{
            pos: other.pos - self.pos,
            rotation: other.rotation - self.rotation,
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

impl Div<f64, Vec2> for Vec2 {
    fn div(self: Vec2, other: f64) -> Vec2 {
        Vec2 {x: self.x / other, y: self.y / other}
    }
}

impl Vec2 {
    pub fn point(&self) -> sdl2::rect::Point {
        sdl2::rect::Point{x: self.x as i32, y: self.y as i32}
    }

    // pub fn rotate_centered(&self, center: &Vec2, rotation: f64) -> Vec2 {
    //     let x_diff = self.x - center.x;
    //     let y_diff = self.y - center.y;
    //     Vec2 {
    //         x: center.x + x_diff * rotation.cos() + y_diff * rotation.sin(),
    //         y: center.y - x_diff * rotation.sin() + y_diff * rotation.cos(),
    //     }
    // }

    // We rotate clockwise because SDL does so too -- the y axes starts
    // from 0 at the top and decreases going down.
    pub fn rotate(&self, rotation: f64) -> Vec2 {
        Vec2 {
            x: self.x * rotation.cos() + self.y * rotation.sin(),
            y: self.y * rotation.cos() - self.x * rotation.sin(),
        }
    }

    pub fn transform(&self, trans: &Transform) -> Vec2 {
        self.rotate(trans.rotation) + trans.pos
    }

    pub fn mag(&self) -> f64 {
        (self.x*self.x + self.y*self.y).sqrt()
    }

    pub fn zero() -> Vec2 {
        Vec2{x: 0., y: 0.}
    }
}

// ---------------------------------------------------------------------
// Rect

#[deriving(PartialEq, Clone, Show, Copy)]
pub struct Rect {
    // The top-left corner of the rectangle.
    pub pos: Vec2,
    pub w: f64,
    pub h: f64,
}

#[inline(always)]
fn min(x: f64, y: f64) -> f64 {
    if x < y { x } else { y }
}

#[inline(always)]
fn max(x: f64, y: f64) -> f64 {
    if x >= y { x } else { y }
}
 
impl Rect {
    pub fn sdl_rect(&self) -> sdl2::rect::Rect {
        sdl2::rect::Rect {
            x: self.pos.x as i32,
            y: self.pos.y as i32,
            w: self.w as i32,
            h: self.h as i32,
        }
    }

    #[inline(always)]
    pub fn transform(&self, trans: &Transform) -> (Vec2, Vec2, Vec2, Vec2) {
        (self.pos.transform(trans),
         (self.pos + Vec2{x: self.w, y: 0.}).transform(trans),
         (self.pos + Vec2{x: 0., y: self.h}).transform(trans),
         (self.pos + Vec2{x: self.w, y: self.h}).transform(trans))
    }

    pub fn overlapping(&this: &Rect, this_t: &Transform, other: &Rect, other_t: &Transform) -> bool {
        #[inline(always)]
        fn project_rect(axis: Vec2, tl: Vec2, tr: Vec2, bl: Vec2, br: Vec2) -> (f64, f64) {
            let (min_1, max_1) = project_edge(axis, tl, tr);
            let (min_2, max_2) = project_edge(axis, tl, bl);
            let (min_3, max_3) = project_edge(axis, bl, br);
            let (min_4, max_4) = project_edge(axis, tr, br);
            (min(min_1, min(min_2, min(min_3, min_4))), max(max_1, max(max_2, max(max_3, max_4))))
        }

        #[inline(always)]
        fn project_edge(axis: Vec2, l: Vec2, r: Vec2) -> (f64, f64) {
            let p1 = project_vec(axis, l);
            let p2 = project_vec(axis, r);
            if p1 < p2 { (p1, p2) } else { (p2, p1) }
        }

        #[inline(always)]
        fn project_vec(u: Vec2, v: Vec2) -> f64 {
            let v_mag = v.mag();
            let cos = (u.x.abs()*v.x + u.y.abs()*v.y) / (u.mag() * v_mag);
            cos*v_mag
        }

        // Get the four corners of each rect.
        let (this_tl, this_tr, this_bl, this_br) = this.transform(this_t);
        let (other_tl, other_tr, other_bl, other_br) = other.transform(other_t);

        // Get the 4 axes.
        let axis_1 = this_tl - this_tr;
        let axis_2 = this_tl - this_bl;
        let axis_3 = other_tl - other_tr;
        let axis_4 = other_tl - other_bl;

        // Get projections.
        let (this_axis_1_min, this_axis_1_max) = project_edge(axis_1, this_tl, this_tr);
        let (this_axis_2_min, this_axis_2_max) = project_edge(axis_2, this_tl, this_bl);
        let (this_axis_3_min, this_axis_3_max) = project_rect(axis_3, this_tl, this_tr, this_bl, this_br);
        let (this_axis_4_min, this_axis_4_max) = project_rect(axis_4, this_tl, this_tr, this_bl, this_br);
        let (other_axis_1_min, other_axis_1_max) = project_rect(axis_1, other_tl, other_tr, other_bl, other_br);
        let (other_axis_2_min, other_axis_2_max) = project_rect(axis_2, other_tl, other_tr, other_bl, other_br);
        let (other_axis_3_min, other_axis_3_max) = project_edge(axis_3, other_tl, other_tr);
        let (other_axis_4_min, other_axis_4_max) = project_edge(axis_4, other_tl, other_bl);

        // If they don't overlap on at least one axis, we're good.
        let separated =
            (this_axis_1_max < other_axis_1_min || other_axis_1_max < this_axis_1_min) ||
            (this_axis_2_max < other_axis_2_min || other_axis_2_max < this_axis_2_min) ||
            (this_axis_3_max < other_axis_3_min || other_axis_3_max < this_axis_3_min) ||
            (this_axis_4_max < other_axis_4_min || other_axis_4_max < this_axis_4_min);
        !separated
    }
}

#[test]
fn test_overlapping() {
    let rect_1 = Rect {
        pos: Vec2{x: -0.5, y: -1.},
        w: 1.,
        h: 2.,
    };
    let rect_2 = Rect {
        pos: Vec2{x: -1., y: -0.5},
        w: 2.,
        h: 1.,
    };
    assert!(Rect::overlapping(&rect_1, &Transform::id(), &rect_2, &Transform::id()));
    assert!(!Rect::overlapping(
        &rect_1, &Transform{pos: Vec2{x: 1.51, y: 0.}, rotation: 0.},
        &rect_2, &Transform::id()));
    assert!(Rect::overlapping(
        &rect_1, &Transform{pos: Vec2{x: 1.51, y: 0.}, rotation: to_radians(-30.)},
        &rect_2, &Transform::id()));
    assert!(!Rect::overlapping(
        &rect_1, &Transform{pos: Vec2{x: 1.51, y: 0.}, rotation: to_radians(-30.)},
        &rect_2, &Transform{pos: Vec2{x: 0., y: 0.}, rotation: to_radians(-30.)}));
}
