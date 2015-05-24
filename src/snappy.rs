use std::io;

use libc::{c_int, size_t};

#[link(name = "snappy")]
extern {
/*
    fn snappy_compress(input: *const u8,
                       input_length: size_t,
                       compressed: *mut u8,
                       compressed_length: *mut size_t) -> c_int;
*/
    fn snappy_uncompress(compressed: *const u8,
                         compressed_length: size_t,
                         uncompressed: *mut u8,
                         uncompressed_length: *mut size_t) -> c_int;
/*
    fn snappy_max_compressed_length(source_length: size_t) -> size_t;
    fn snappy_uncompressed_length(compressed: *const u8,
                                  compressed_length: size_t,
                                  result: *mut size_t) -> c_int;
    fn snappy_validate_compressed_buffer(compressed: *const u8,
                                         compressed_length: size_t) -> c_int;
*/
}

pub fn uncompress(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    unsafe {
        let srclen = src.len() as size_t;
        let psrc = src.as_ptr();
        let pdst = dst.as_mut_ptr();
        let mut dstlen = dst.capacity() as size_t;
        let r = snappy_uncompress(psrc, srclen, pdst, &mut dstlen);
        if r == 0 {
            dst.set_len(dstlen as usize);
            Ok( () )
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Error in snappy"))
        }
    }
}

