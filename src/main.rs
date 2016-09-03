extern crate sdl2;
extern crate sdl2_image;

use std::collections::HashMap;
use std::path::Path;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

fn init_sdl(vsync: bool) -> sdl2::render::Renderer<'static> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("Dogfights", SCREEN_WIDTH, SCREEN_HEIGHT)
            .position_centered()
            .opengl()
            .allow_highdpi()
            .build()
            .unwrap();
    let renderer_builder0 = window.renderer();
    let renderer_builder = if vsync { renderer_builder0.present_vsync() } else { renderer_builder0 };
    renderer_builder.accelerated().build().unwrap()
}

type TexturesId = &'static str;
type Textures = HashMap<TexturesId, sdl2::render::Texture>;

const PLANES_TEXTURE_ID: TexturesId = "plane";
const MAP_TEXTURE_ID: TexturesId = "map";

fn init_textures<'a>(renderer: &sdl2::render::Renderer<'a>) -> Textures {
    let mut textures = HashMap::new();

    let mut planes_surface: sdl2::surface::Surface =
            sdl2_image::LoadSurface::from_file(Path::new("assets/planes.png")).ok().unwrap();
    planes_surface.set_color_key(true, sdl2::pixels::Color::RGB(0xba, 0xfe, 0xca)).ok().unwrap();
    let planes_texture = renderer.create_texture_from_surface(&planes_surface).ok().unwrap();
    let _ = textures.insert(PLANES_TEXTURE_ID, planes_texture);

    let map_surface: sdl2::surface::Surface = sdl2_image::LoadSurface::from_file(Path::new("assets/background.png")).ok().unwrap();
    let map_texture = renderer.create_texture_from_surface(&map_surface).ok().unwrap();
    let _ = textures.insert(MAP_TEXTURE_ID, map_texture);

    textures
}

fn main() {
    let renderer = init_sdl(true);
    let _textures = init_textures(&renderer);
    println!("Hello, world!");
}
