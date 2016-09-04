#![crate_name = "dogfights"]
#![crate_type = "lib"]
extern crate rustc_serialize; // Why do I need this here? I get an error otherwise.

pub mod vec;
pub mod spec;
pub mod texture;
pub mod transformation;