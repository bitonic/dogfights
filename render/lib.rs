extern crate sdl2;
extern crate "rustc-serialize" as rustc_serialize;

extern crate geometry;
extern crate specs;
extern crate actors;
extern crate conf;

use sdl2::SdlResult;
use sdl2::render::Renderer;
use std::ops::Deref;

use geometry::*;
use specs::*;
use actors::*;
use conf::*;

pub struct RenderEnv {
    pub textures: Textures,
    pub renderer: Renderer,
}

impl RenderEnv {
    fn sprite(&self, sprite: &Sprite, trans: &Transform) -> SdlResult<()> {
        let texture = self.textures.get(&sprite.texture).unwrap();
        let dst = Rect{
            pos: trans.pos - sprite.center,
            w: sprite.rect.w,
            h: sprite.rect.h
        };
        let angle = from_radians(trans.rotation);
        self.renderer.copy_ex(
            texture, Some(sprite.rect.sdl_rect()), Some(dst.sdl_rect()), ((sprite.angle - angle) as f64),
            Some(sprite.center.point()), sdl2::render::RendererFlip::None)
    }

    fn map(&self, map: &Map, pos: &Vec2) -> SdlResult<()> {
        let background_texture = self.textures.get(&map.background_texture).unwrap();

        // Fill the whole screen with the background color
        try!(self.renderer.set_draw_color(map.background_color.to_sdl_color()));
        let rect = sdl2::rect::Rect {
            x: 0, y: 0, w: SCREEN_WIDTH as i32, h: SCREEN_HEIGHT as i32
        };
        try!(self.renderer.fill_rect(&rect));

        // Fill with the background texture.  The assumption is that 4
        // background images are needed to cover the entire screen:
        // 
        // map
        // ┌──────────────────────────────────────────┐
        // │                  ┊                   ┊   │
        // │  pos             ┊                   ┊   │
        // │  ┌─────────────────────┐             ┊   │
        // │  │               ┊     │             ┊   │
        // │  │             t ┊     │             ┊   │
        // │┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄│
        // │  │               ┊     │             ┊   │
        // │  └─────────────────────┘             ┊   │
        // │                  ┊                   ┊   │
        // └──────────────────────────────────────────┘

        let bgr = try!(background_texture.query());
        let bgr_w = bgr.width as f32;
        let bgr_h = bgr.height as f32;
        let t = Vec2 {
            x: bgr_w - (pos.x % bgr_w),
            y: bgr_h - (pos.y % bgr_h),
        };
        let top_left = Vec2 {
            x: t.x - bgr_w,
            y: t.y - bgr_h,
        };
        let top_right = Vec2 {
            x: t.x,
            y: t.y - bgr_h,
        };
        let bottom_left = Vec2 {
            x: t.x - bgr_w,
            y: t.y,
        };
        let bottom_right = Vec2 {
            x: t.x,
            y: t.y,
        };
        let to_rect = |&: p: Vec2| -> Option<sdl2::rect::Rect> {
            Some(sdl2::rect::Rect {
                x: p.x as i32,
                y: p.y as i32,
                w: bgr.width as i32,
                h: bgr.height as i32,
            })
        };
        
        try!(self.renderer.copy(background_texture, None, to_rect(top_left)));
        try!(self.renderer.copy(background_texture, None, to_rect(top_right)));
        try!(self.renderer.copy(background_texture, None, to_rect(bottom_left)));
        try!(self.renderer.copy(background_texture, None, to_rect(bottom_right)));
        Ok(())
    }

    fn actor(&self, actor: &Actor, sspec: &GameSpec, trans: &Transform) -> SdlResult<()> {
        match *actor {
            Actor::Ship(ref ship) => self.ship(ship, sspec, trans),
            Actor::Shooter(ref shooter) => self.shooter(shooter, sspec, trans),
            Actor::Bullet(ref bullet) => self.bullet(bullet, sspec, trans),
        }
    }

    fn bullet(&self, bullet: &Bullet, sspec: &GameSpec, trans: &Transform) -> SdlResult<()> {
        let spec = sspec.get_spec(bullet.spec).is_bullet();
        let trans = trans.adjust(&bullet.trans);
        try!(self.sprite(&spec.sprite, &trans));
        // Debugging -- render bbox
        self.bbox(&spec.bbox, &trans)
    }

    fn bbox(&self, bbox: &BBox,trans: &Transform) -> SdlResult<()> {
        try!(self.renderer.set_draw_color(sdl2::pixels::Color::RGB(0xFF, 0x00, 0x00)));
        for rect in bbox.rects.iter() {
            let (tl, tr, bl, br) = rect.transform(trans);
            try!(self.renderer.draw_line(tl.point(), tr.point()));
            try!(self.renderer.draw_line(tr.point(), br.point()));
            try!(self.renderer.draw_line(br.point(), bl.point()));
            try!(self.renderer.draw_line(bl.point(), tl.point()));
        };
        Ok(())
    }

    fn ship(&self, ship: &Ship, sspec: &GameSpec, trans: &Transform) -> SdlResult<()> {
        let trans = trans.adjust(&ship.trans);
        let spec = sspec.get_spec(ship.spec).is_ship();

        // =============================================================
        // Render ship
        if ship.accel {
            try!(self.sprite(&spec.sprite_accel, &trans));
        } else {
            try!(self.sprite(&spec.sprite, &trans));
        }

        // =============================================================
        // Debugging -- render bbox
        self.bbox(&spec.bbox, &trans)
    }

    fn shooter(&self, shooter: &Shooter, sspec: &GameSpec, trans: &Transform) -> SdlResult<()> {
        let spec = sspec.get_spec(shooter.spec).is_shooter();
        self.sprite(&spec.sprite, &trans.adjust(&spec.trans))
    }

    fn actors(&self, actors: &Actors, spec: &GameSpec, trans: &Transform) -> SdlResult<()> {
        try!(self.map(&spec.map, &trans.pos));
        for actor in actors.values() {
            try!(self.actor(actor, spec, trans));
        };
        Ok(())
    }

    pub fn game(&self, game: &Game, spec: &GameSpec, player: ActorId) -> SdlResult<()> {
        let trans = &game.actors.get(player).unwrap().is_ship().camera.transform();
        try!(self.map(&spec.map, &trans.pos));
        try!(self.actors(&game.actors, spec, trans));
        Ok(())
    }

    pub fn player_game(&self, game: &PlayerGame, spec: &GameSpec) -> SdlResult<()> {
        self.game(game.game.deref(), spec, game.player)
    }
}
