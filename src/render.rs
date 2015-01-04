extern crate sdl2;
extern crate "rustc-serialize" as rustc_serialize;

use sdl2::SdlResult;
use sdl2::render::Renderer;
use sdl2::pixels::Color;

use geometry::*;
use specs::*;
use actors::*;
use constants::*;

fn render_sprite(sprite: &Sprite, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    let dst = Rect{
        pos: trans.pos - sprite.center,
        w: sprite.rect.w,
        h: sprite.rect.h
    };
    let angle = from_radians(trans.rotation);
    renderer.copy_ex(
        sprite.texture, Some(sprite.rect.sdl_rect()), Some(dst.sdl_rect()), ((sprite.angle - angle) as f64),
        Some(sprite.center.point()), sdl2::render::RendererFlip::None)
}

pub fn render_map(map: &Map, renderer: &Renderer, pos: &Vec2) -> SdlResult<()> {
    // Fill the whole screen with the background color
    try!(renderer.set_draw_color(map.background_color));
    let rect = sdl2::rect::Rect {
        x: 0, y: 0, w: SCREEN_WIDTH as i32, h: SCREEN_HEIGHT as i32
    };
    try!(renderer.fill_rect(&rect));

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

    let bgr = try!(map.background_texture.query());
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
    let to_rect = |p: Vec2| -> Option<sdl2::rect::Rect> {
        Some(sdl2::rect::Rect {
            x: p.x as i32,
            y: p.y as i32,
            w: bgr.width as i32,
            h: bgr.height as i32,
        })
    };
    
    try!(renderer.copy(map.background_texture, None, to_rect(top_left)));
    try!(renderer.copy(map.background_texture, None, to_rect(top_right)));
    try!(renderer.copy(map.background_texture, None, to_rect(bottom_left)));
    renderer.copy(map.background_texture, None, to_rect(bottom_right))
}

fn render_actor(actor: &Actor, sspec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    match *actor {
        Actor::Ship(ref ship) => render_ship(ship, sspec, renderer, trans),
        Actor::Shooter(ref shooter) => render_shooter(shooter, sspec, renderer, trans),
        Actor::Bullet(ref bullet) => render_bullet(bullet, sspec, renderer, trans),
    }
}

fn render_bullet(bullet: &Bullet, sspec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    let spec = sspec.get_spec(bullet.spec).is_bullet();
    let trans = trans.adjust(&bullet.trans);
    try!(render_sprite(spec.sprite, renderer, &trans));
    // Debugging -- render bbox
    render_bbox(spec.bbox, renderer, &trans)
}

fn render_bbox(bbox: &BBox, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    try!(renderer.set_draw_color(Color::RGB(0xFF, 0x00, 0x00)));
    for rect in bbox.rects.iter() {
        let (tl, tr, bl, br) = rect.transform(trans);
        try!(renderer.draw_line(tl.point(), tr.point()));
        try!(renderer.draw_line(tr.point(), br.point()));
        try!(renderer.draw_line(br.point(), bl.point()));
        try!(renderer.draw_line(bl.point(), tl.point()));
    };
    Ok(())
}

fn render_ship(ship: &Ship, sspec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    let trans = trans.adjust(&ship.trans);
    let spec = sspec.get_spec(ship.spec).is_ship();

    // =============================================================
    // Render ship
    if ship.accel {
        try!(render_sprite(spec.sprite_accel, renderer, &trans));
    } else {
        try!(render_sprite(spec.sprite, renderer, &trans));
    }

    // =============================================================
    // Debugging -- render bbox
    render_bbox(spec.bbox, renderer, &trans)
}

fn render_shooter(shooter: &Shooter, sspec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    let spec = sspec.get_spec(shooter.spec).is_shooter();
    render_sprite(spec.sprite, renderer, &trans.adjust(&spec.trans))
}

pub fn render_actors(actors: &Actors, spec: &GameSpec, renderer: &Renderer, trans: &Transform) -> SdlResult<()> {
    try!(render_map(spec.map, renderer, &trans.pos));
    for actor in actors.actors.values() {
        try!(render_actor(actor, spec, renderer, trans));
    };
    Ok(())
}
