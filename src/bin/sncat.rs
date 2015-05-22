extern crate snzip;

use std::io;

fn main() {
    let mut dec = snzip::Decompressor::new(io::stdin());
    io::copy(&mut dec, &mut io::stdout()).unwrap();
}
