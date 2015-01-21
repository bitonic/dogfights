#![allow(unstable)]
extern crate dogfights;
extern crate getopts;

use getopts::{optopt, optflag, getopts, usage, OptGroup};
use std::str::FromStr;

fn print_usage(program: String, opts: &[OptGroup]) {
    std::io::println(usage(program.as_slice(), opts).as_slice());
}

fn main() {
    let args = std::os::args();
    let program = args[0].clone();

    let opts = &[
        optopt("s", "server", "Server to connect to", "ADDRESS"),
        optopt("p", "port", "The port to bind to", "PORT"),
        optopt("", "ai", "AI to use", "AI"),
        optflag("x", "display", "Whether to show a display or not")
    ];
    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    let server = match matches.opt_str("s") {
        None    => {print_usage(program, opts); return;}
        Some(s) => s,
    };
    let port: u16 = match matches.opt_str("p") {
        None    => {print_usage(program, opts); return;}
        Some(s) => match FromStr::from_str(s.as_slice()) {
            None    => {print_usage(program, opts); return;}
            Some(p) => p
        },
    };
    let ai_s = match matches.opt_str("ai") {
        None    => {print_usage(program, opts); return;}
        Some(s) => s,
    };
    let display = matches.opt_present("x");
    dogfights::run_remote_ai(&*server, ("127.0.0.1", port), &*ai_s, display)
}
