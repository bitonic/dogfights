extern crate sdl2;
extern crate sdl2_image;

use sdl2::render::Renderer;
use std::collections::HashMap;

use conf::*;
use geometry::*;
use specs::*;

const PLANES_TEXTURE_ID: TextureId = 0;
const MAP_TEXTURE_ID: TextureId = 1;

pub fn init_sdl(vsync: bool) -> Renderer {
    sdl2::init(sdl2::INIT_EVERYTHING | sdl2::INIT_TIMER);
    let window = sdl2::video::Window::new(
        "Dogfights",
        sdl2::video::WindowPos::PosUndefined, sdl2::video::WindowPos::PosUndefined,
        (SCREEN_WIDTH as isize), (SCREEN_HEIGHT as isize),
        sdl2::video::SHOWN).ok().unwrap();
    let vsync_flag = if vsync { sdl2::render::PRESENTVSYNC } else { sdl2::render::RendererFlags::empty() };
    let flags = sdl2::render::ACCELERATED | vsync_flag;
    let renderer = Renderer::from_window(window, sdl2::render::RenderDriverIndex::Auto, flags).ok().unwrap();
    renderer.set_logical_size((SCREEN_WIDTH as isize), (SCREEN_HEIGHT as isize)).ok().unwrap();
    renderer
}

pub fn init_headless_sdl() {
    sdl2::init(sdl2::INIT_TIMER);
}

pub fn init_textures(renderer: &Renderer) -> Textures {
    let mut textures = HashMap::new();

    let planes_surface: sdl2::surface::Surface =
        sdl2_image::LoadSurface::from_file(&("assets/planes.png".parse()).unwrap()).ok().unwrap();
    planes_surface.set_color_key(true, sdl2::pixels::Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture = renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let _ = textures.insert(PLANES_TEXTURE_ID, planes_texture);

    let map_surface = sdl2_image::LoadSurface::from_file(&("assets/background.png".parse()).unwrap()).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let _ = textures.insert(MAP_TEXTURE_ID, map_texture);

    textures
}

pub fn init_spec() -> GameSpec {
    // Specs
    let mut specs = Vec::new();
    let bullet_spec = BulletSpec{
        sprite: Sprite{
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 424., y: 140.}, w: 3., h: 12.},
            center: Vec2{x: 1., y: 6.},
            angle: 90.,
        },
        vel: 1000.,
        lifetime: 5000.,
        bbox: BBox{
            rects: vec![
                Rect{
                    pos: Vec2{y: -1.5, x: -6.},
                    h: 3.,
                    w: 12.
                }]
        },
    };
    let bullet_spec_id = 0;
    specs.push(Spec::BulletSpec(bullet_spec));
    let ship_spec = ShipSpec{
        rotation_vel: 10.,
        rotation_vel_accel: 1.,
        accel: 800.,
        friction: 1.,
        gravity: 100.,
        sprite: Sprite{
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 128., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        sprite_accel: Sprite{
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 88., y: 96.}, w: 30., h: 24.},
            center: Vec2{x: 15., y: 12.},
            angle: 90.,
        },
        bullet_spec: bullet_spec_id,
        firing_interval: 1.,
        shoot_from: Vec2{x: 18., y: 0.},
        bbox: BBox{
            rects: vec![
                Rect{
                    pos: Vec2{x: -12., y: -5.5},
                    w: 25.,
                    h: 11.
                },
                Rect{
                    pos: Vec2{x: 0., y: -15.},
                    w: 7.5,
                    h: 30.
                }
                ]
        },
    };
    let ship_spec_id: SpecId = 1;
    specs.push(Spec::ShipSpec(ship_spec));
    let shooter_spec = ShooterSpec {
        sprite: Sprite{
            texture: PLANES_TEXTURE_ID,
            rect: Rect{pos: Vec2{x: 48., y: 248.}, w: 32., h: 24.},
            center: Vec2{x: 16., y: 12.},
            angle: 90.,
        },
        trans: Transform {
            pos: Vec2{x: 1000., y: 200.},
            rotation: to_radians(270.),
        },
        bullet_spec: bullet_spec_id,
        firing_rate: 2.,
    };
    let shooter_spec_id: SpecId = 2;
    specs.push(Spec::ShooterSpec(shooter_spec));
    let map = Map {
        w: SCREEN_WIDTH*10.,
        h: SCREEN_HEIGHT*10.,
        background_color: Color(0x58, 0xB7, 0xFF),
        background_texture: MAP_TEXTURE_ID,
    };
    let camera_spec = CameraSpec {
        accel: 1.2,
        h_pad: 220.,
        v_pad: 220. * SCREEN_HEIGHT / SCREEN_WIDTH,
    };
    GameSpec{
        map: map,
        camera_spec: camera_spec,
        ship_spec: ship_spec_id,
        shooter_spec: shooter_spec_id,
        specs: specs,
    }
}
