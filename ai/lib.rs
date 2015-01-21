#![allow(unstable)]
extern crate actors;
extern crate input;

use std::str::FromStr;

use actors::*;
use input::*;

pub trait Ai {
    fn move_(&self, game: &PlayerGame) -> Input;
}

#[derive(Copy, PartialEq, Clone)]
pub struct Follower {
    following: ActorId
}

impl Follower {
    pub fn new(following: ActorId) -> Follower {
        Follower{following: following}
    }
}

impl Ai for Follower {
    fn move_(&self, _game: &PlayerGame) -> Input {
        Input::new()
    }
}

pub fn parse_ai_string(s: &str, player: Option<ActorId>) -> Box<Ai + Send + 'static> {
    if s.starts_with("follower") {
        if s == "follower" {
            match player {
                None => panic!("follower AI with no player (no default player provided)"),
                Some(player) => Box::new(Follower::new(player)),
            }
        } else {
            let segments: Vec<&str> = s.split(':').collect();
            if !(segments.len() == 2 && segments[0] == "follower") {
                panic!("Malformed AI string");
            } else {
                match FromStr::from_str(segments[1]) {
                    None => panic!("Malformed AI string"),
                    Some(player) => Box::new(Follower::new(player)),
                }
            }
        }
    } else {
        panic!("Malformed AI string")
    }
}

#[test]
fn test_parse() {
    parse_ai_string("follower", Some(0));
    parse_ai_string("follower:3", None);
}
