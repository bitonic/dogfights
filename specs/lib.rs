extern crate sdl2;

extern crate geometry;

use std::collections::HashMap;
use sdl2::pixels::Color;
use sdl2::render::Texture;

use geometry::*;

// ---------------------------------------------------------------------
// Textures

pub type TextureId = u32;
pub type Textures = HashMap<TextureId, Texture>;

// ---------------------------------------------------------------------
// Sprites

#[derive(PartialEq, Clone, Copy)]
pub struct Sprite {
    pub texture: TextureId,
    pub rect: Rect,
    pub center: Vec2,
    // If the sprite is already rotated by some angle
    pub angle: f32,
}

// ---------------------------------------------------------------------
// Map

#[derive(PartialEq, Clone, Copy)]
pub struct Map {
    pub w: f32,
    pub h: f32,
    pub background_color: Color, 
    pub background_texture: TextureId,
}

impl Map {
    pub fn bound(&self, p: Vec2) -> Vec2 {
        // TODO handle points that are badly negative
        fn f(n: f32, m: f32) -> f32 {
            if n < 0. {
                0.
            } else if n > m {
                m
            } else {
                n
            }
        };
        Vec2{x: f(p.x, self.w), y: f(p.y, self.h)}
    }

    pub fn bound_rect(&self, p: Vec2, w: f32, h: f32) -> Vec2 {
        fn f(n: f32, edge: f32, m: f32) -> f32 {
            if n < 0. {
                0.
            } else if n + edge > m {
                m - edge
            } else {
                n
            }
        };
        Vec2{x: f(p.x, w, self.w), y: f(p.y, h, self.h)}
    }
}

// ---------------------------------------------------------------------
// BBox

#[derive(PartialEq, Clone)]
pub struct BBox {
    pub rects: Vec<Rect>,
}

impl BBox {
    pub fn overlapping(this: BBox, this_t: &Transform, other: BBox, other_t: &Transform) -> bool {
        let mut overlap = false;
        for this in this.rects.iter() {
            if overlap { break };
            for other in other.rects.iter() {
                if overlap { break };
                overlap = Rect::overlapping(this, this_t, other, other_t);
            }
        }
        overlap
    }
}

// ---------------------------------------------------------------------
// Specs

pub type SpecId = u32;

#[derive(PartialEq, Clone, Show, Copy)]
pub struct CameraSpec {
    pub accel: f32,
    // The minimum distance from the top/bottom edges to the ship
    pub v_pad: f32,
    // The minimum distance from the left/right edges to the ship
    pub h_pad: f32,
}

#[derive(PartialEq, Clone)]
pub struct ShipSpec {
    pub rotation_vel: f32,
    pub rotation_vel_accel: f32,
    pub accel: f32,
    pub friction: f32,
    pub gravity: f32,
    pub sprite: Sprite,
    pub sprite_accel: Sprite,
    pub bullet_spec: SpecId,
    pub firing_interval: f32,
    pub shoot_from: Vec2,
    pub bbox: BBox,
}

#[derive(PartialEq, Clone)]
pub struct BulletSpec {
    pub sprite: Sprite,
    pub vel: f32,
    pub lifetime: f32,
    pub bbox: BBox,
}

#[derive(PartialEq, Clone, Copy)]
pub struct ShooterSpec {
    pub sprite: Sprite,
    pub trans: Transform,
    pub bullet_spec: SpecId,
    pub firing_rate: f32,
}

#[derive(PartialEq, Clone)]
pub enum Spec {
    ShipSpec(ShipSpec),
    ShooterSpec(ShooterSpec),
    BulletSpec(BulletSpec),
}

impl Spec {
    pub fn is_ship(&self) -> &ShipSpec {
        match *self {
            Spec::ShipSpec(ref spec) => spec,
            _                        => unreachable!(),
        }
    }

    pub fn is_shooter(&self) -> &ShooterSpec {
        match *self {
            Spec::ShooterSpec(ref spec) => spec,
            _                           => unreachable!(),
        }
    }

    pub fn is_bullet(&self) -> &BulletSpec {
        match *self {
            Spec::BulletSpec(ref spec) => spec,
            _                          => unreachable!(),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct GameSpec {
    pub map: Map,
    pub camera_spec: CameraSpec,
    pub ship_spec: SpecId,
    pub shooter_spec: SpecId,
    pub specs: Vec<Spec>,
}

impl GameSpec {
    pub fn get_spec(&self, spec_id: SpecId) -> &Spec {
        &self.specs[spec_id as usize]
    }
}
