extern crate snzip;

use std::io;
use std::env;

use snzip::framing::Decompressor;

fn main() {
    let args:Vec<_> = env::args().collect();
    let fast = args.len() > 1 && args[1] == "--fast";
    let mut dec = Decompressor::new(io::stdin()).fast(fast);
    io::copy(&mut dec, &mut io::stdout()).unwrap();
}
