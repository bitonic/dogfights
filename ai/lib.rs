extern crate actors;
extern crate input;

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

// pub fn attach<T: Ai>(handle: &server::Client) -> Sender<()> {
//     let player = handle.join();
//     let (quit_tx, quit_rx) = channel();

//     let _ = Thread::spawn(move || {
        
//     });

//     quit_tx
// }
