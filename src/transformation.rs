use std::ops::{Add, Sub, Mul, Div};

use vec::Vec2;

#[derive(PartialEq, Clone, Copy, RustcEncodable, RustcDecodable)]
pub struct Transformation {
    ix11: f32, ix12: f32, ix13: f32,
    ix21: f32, ix22: f32, ix23: f32,
    ix31: f32, ix32: f32, ix33: f32
}

impl Transformation {
    #[inline]
    fn new(ix11: f32, ix12: f32, ix13: f32, ix21: f32, ix22: f32, ix23: f32, ix31: f32, ix32: f32, ix33: f32) -> Transformation {
        Transformation{
            ix11: ix11, ix12: ix12, ix13: ix13,
            ix21: ix21, ix22: ix22, ix23: ix23,
            ix31: ix31, ix32: ix32, ix33: ix33
        }
    }
    #[inline]
    pub fn identity() -> Transformation {
        Transformation::new(
            1., 0., 0.,
            0., 1., 0.,
            0., 0., 1.
        )
    }

    #[inline]
    /// We rotate clockwise because SDL does so too -- the y axes starts
    /// from 0 at the top and decreases going down.
    pub fn rotation(rot: f32) -> Transformation {
        Transformation::new(
            rot.cos(),  rot.sin(), 0.,
            -rot.sin(), rot.cos(), 0.,
            0.,         0.,        1.
        )
    }

    #[inline]
    pub fn rotation_about(rot: f32, point: Vec2) -> Transformation {
        Transformation::translation(point) * Transformation::rotation(rot) * Transformation::translation(-point)
    }

    #[inline]
    pub fn translation(v: Vec2) -> Transformation {
        Transformation::new(
            1., 0., v.x,
            0., 1., v.y,
            0., 0., 1.
        )
    }

    #[inline]
    pub fn apply_to(self, v: Vec2) -> Vec2 {
        Vec2{
            x: self.ix11*v.x + self.ix12*v.y + self.ix13,
            y: self.ix21*v.x + self.ix22*v.y + self.ix23
        }
    }
}

impl Mul<Transformation> for Transformation {
    type Output = Transformation;

    #[inline]
    fn mul(self, other: Transformation) -> Transformation {
        Transformation::new(
            self.ix11*other.ix11 + self.ix12*other.ix21 + self.ix13*other.ix31,
            self.ix11*other.ix12 + self.ix12*other.ix22 + self.ix13*other.ix32,
            self.ix11*other.ix13 + self.ix12*other.ix23 + self.ix13*other.ix33,

            self.ix21*other.ix11 + self.ix22*other.ix21 + self.ix23*other.ix31,
            self.ix21*other.ix12 + self.ix22*other.ix22 + self.ix23*other.ix32,
            self.ix21*other.ix13 + self.ix22*other.ix23 + self.ix23*other.ix33,

            self.ix31*other.ix11 + self.ix32*other.ix21 + self.ix33*other.ix31,
            self.ix31*other.ix12 + self.ix32*other.ix22 + self.ix33*other.ix32,
            self.ix31*other.ix13 + self.ix32*other.ix23 + self.ix33*other.ix33
        )
    }
}

impl Mul<f32> for Transformation {
    type Output = Transformation;

    #[inline]
    fn mul(self, other: f32) -> Transformation {
        Transformation::new(
            self.ix11*other, self.ix12*other, self.ix13*other,
            self.ix21*other, self.ix22*other, self.ix23*other,
            self.ix31*other, self.ix32*other, self.ix33*other
        )
    }
}

impl Add for Transformation {
    type Output = Transformation;

    #[inline]
    fn add(self, other: Transformation) -> Transformation {
        Transformation::new(
            self.ix11+other.ix11, self.ix12+other.ix12, self.ix13+other.ix13,
            self.ix21+other.ix21, self.ix22+other.ix22, self.ix23+other.ix23,
            self.ix31+other.ix31, self.ix32+other.ix32, self.ix33+other.ix33
        )
    }
}

impl Sub for Transformation {
    type Output = Transformation;

    #[inline]
    fn sub(self, other: Transformation) -> Transformation {
        Transformation::new(
            self.ix11-other.ix11, self.ix12-other.ix12, self.ix13-other.ix13,
            self.ix21-other.ix21, self.ix22-other.ix22, self.ix23-other.ix23,
            self.ix31-other.ix31, self.ix32-other.ix32, self.ix33-other.ix33
        )
    }
}

impl Div<f32> for Transformation {
    type Output = Transformation;

    #[inline]
    fn div(self, other: f32) -> Transformation {
        Transformation::new(
            self.ix11/other, self.ix12/other, self.ix13/other,
            self.ix21/other, self.ix22/other, self.ix23/other,
            self.ix31/other, self.ix32/other, self.ix33/other
        )
    }
}
