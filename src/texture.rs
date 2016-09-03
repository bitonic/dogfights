extern crate sdl2;

use std::collections::HashMap;

// Textures
// --------------------------------------------------------------------

pub type TextureId = u32;
pub type Textures = HashMap<TextureId, sdl2::render::Texture>;
