extern crate snzip;
extern crate rustc_serialize;
extern crate docopt;

use docopt::Docopt;

use std::io;

use snzip::framing::{ Compressor, Decompressor };

static USAGE: &'static str = "
Usage: sncat [--fast] [-d]

Options:
    --fast              Dont'check CRC.
    -d, --decompress    Decompress
";

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_fast: bool,
    flag_decompress: bool
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                    .and_then(|d| d.decode())
                    .unwrap_or_else(|e| e.exit());
    if args.flag_decompress {
        let mut dec = Decompressor::new(io::stdin()).fast(args.flag_fast);
        io::copy(&mut dec, &mut io::stdout()).unwrap();
    } else {
        let mut comp = Compressor::new(io::stdout());
        io::copy(&mut io::stdin(), &mut comp).unwrap();
    }
}
