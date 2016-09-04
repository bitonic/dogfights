extern crate sdl2;

use transformation::Transformation;
use vec::Vec2;

// ---------------------------------------------------------------------
// Rect

#[derive(PartialEq, Clone, Copy, RustcDecodable, RustcEncodable)]
pub struct Rect {
    // The top-left corner of the rectangle.
    pub pos: Vec2,
    pub w: f32,
    pub h: f32,
}

#[inline(always)]
fn min(x: f32, y: f32) -> f32 {
    if x < y { x } else { y }
}

#[inline(always)]
fn max(x: f32, y: f32) -> f32 {
    if x >= y { x } else { y }
}
 
impl Rect {
    pub fn sdl_rect(&self) -> sdl2::rect::Rect {
        sdl2::rect::Rect::new(
            self.pos.x as i32,
            self.pos.y as i32,
            self.w as u32,
            self.h as u32
        )
    }

    #[inline(always)]
    pub fn transform(self, trans: Transformation) -> (Vec2, Vec2, Vec2, Vec2) {
        (
            self.pos.transform(trans),
            self.pos + Vec2{x: self.w, y: 0.}.transform(trans),
            self.pos + Vec2{x: 0., y: self.h}.transform(trans),
            self.pos + Vec2{x: self.w, y: self.h}.transform(trans)
        )
    }

    pub fn overlapping(&this: &Rect, this_t: &Transformation, other: &Rect, other_t: &Transformation) -> bool {
        #[inline(always)]
        fn project_rect(axis: Vec2, tl: Vec2, tr: Vec2, bl: Vec2, br: Vec2) -> (f32, f32) {
            let (min_1, max_1) = project_edge(axis, tl, tr);
            let (min_2, max_2) = project_edge(axis, tl, bl);
            let (min_3, max_3) = project_edge(axis, bl, br);
            let (min_4, max_4) = project_edge(axis, tr, br);
            (min(min_1, min(min_2, min(min_3, min_4))), max(max_1, max(max_2, max(max_3, max_4))))
        }

        #[inline(always)]
        fn project_edge(axis: Vec2, l: Vec2, r: Vec2) -> (f32, f32) {
            let p1 = project_vec(axis, l);
            let p2 = project_vec(axis, r);
            if p1 < p2 { (p1, p2) } else { (p2, p1) }
        }

        #[inline(always)]
        fn project_vec(u: Vec2, v: Vec2) -> f32 {
            let v_mag = v.mag();
            let cos = (u.x.abs()*v.x + u.y.abs()*v.y) / (u.mag() * v_mag);
            cos*v_mag
        }

        // Get the four corners of each rect.
        let (this_tl, this_tr, this_bl, this_br) = this.transform(*this_t);
        let (other_tl, other_tr, other_bl, other_br) = other.transform(*other_t);

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
