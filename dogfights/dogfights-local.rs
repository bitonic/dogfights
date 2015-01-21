#![allow(unstable)]
extern crate dogfights;
extern crate getopts;

use getopts::{optopt, optflag, getopts, usage, OptGroup};
use std::str::FromStr;

fn main() {
    // let args = std::os::args();
    // let program = args[0].clone();

    // let opts = &[
    //     optmulti("", "ai", "Add an AI to the game", "AI"),
    // ];
    // let matches = match getopts(args.tail(), opts) {
    //     Ok(m) => m,
    //     Err(f) => panic!(f.to_string()),
    // };

    // let ais: Vec<String> = matches.opt_strs("ai");

    dogfights::run_local(Vec::new())
}
