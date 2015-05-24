extern crate byteorder;
extern crate crc;
extern crate libc;

use std::io;
use std::io::prelude::*;

use std::fmt;

use byteorder::{ ByteOrder, ReadBytesExt, LittleEndian };

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

fn uncompress(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
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

#[derive(Debug)]
enum ChunkType {
    StreamIdentifier,
    CompressedData,
    RawData,
}

pub struct Decompressor<R : Read> {
    check_crc: bool,
    check_stream_identifier:bool,
    inner:R,
    chunk:Option<ChunkType>,
    buf:Vec<u8>,
    buf_dec:Vec<u8>,
    position: usize,
}

impl<R : Read> fmt::Debug for Decompressor<R> {

    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decompressor: {:?} {:?}", &self.buf.len(), &self.chunk)
    }

}

impl<R : Read> Decompressor<R> {

    pub fn new(r:R) -> Decompressor<R> {
        Decompressor { inner:r, buf: Vec::new(), buf_dec:Vec::with_capacity(65536),
            chunk:None, check_crc: true, check_stream_identifier: true,
            position: 0
        }
    }

    pub fn check_crc(self, v:bool) -> Decompressor<R> {
        Decompressor { check_crc:v, .. self }
    }

    pub fn check_stream_identifier(self, v:bool) -> Decompressor<R> {
        Decompressor { check_stream_identifier:v, .. self }
    }

    pub fn fast(self, v:bool) -> Decompressor<R> {
        Decompressor { check_stream_identifier:!v, check_crc:!v, .. self }
    }

    fn checksum(buf:&[u8]) -> u32 {
        let crc:u32 = crc::crc32::checksum_castagnoli(buf);
        ((crc >> 15) | (crc << 17)).wrapping_add(0xa282ead8)
    }

    fn load_chunk(&mut self) -> Result<(), io::Error> {
        let mut kind:[u8;1] = [0];
        if try!(self.inner.read(&mut kind)) == 0 {
            return Ok(())
        }
        let size:usize = try!(self.inner.read_uint::<LittleEndian>(3)) as usize;
        self.buf.clear();
        if size > self.buf.capacity() {
            self.buf.reserve(size);
        }
        unsafe { self.buf.set_len(size); }
        let mut read = 0;
        while read < size {
            let more = try!(self.inner.read(&mut self.buf[read ..]));
            if more == 0 {
                return Err(io::Error::new(io::ErrorKind::Other, "incomplete page"));
            }
            read += more;
        }
        if kind[0] == 0xff {
            if !self.check_stream_identifier || self.buf == "sNaPpY".as_bytes() {
                self.chunk = Some(ChunkType::StreamIdentifier);
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "invalid sNaPpY header"))
            }
        } else if kind[0] == 0x00 {
            let check = LittleEndian::read_u32(&self.buf);
            try!(uncompress(&self.buf[4 ..], &mut self.buf_dec));
            if self.check_crc && check != Self::checksum(&self.buf_dec) {
                return Err(io::Error::new(io::ErrorKind::Other, "invalid crc for snappy page"))
            } else {
                self.position = 0;
                self.chunk = Some(ChunkType::CompressedData);
            }
        } else if kind[0] == 0x01 {
            let check = LittleEndian::read_u32(&*self.buf);
            if self.check_crc && check != Self::checksum(&self.buf[4..]) {
                return Err(io::Error::new(io::ErrorKind::Other, "invalid crc for snappy page"))
            } else {
                self.position = 4;
                self.chunk = Some(ChunkType::RawData);
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "unknown page type"));
        }
        Ok( () )
    }
}

impl<R : Read> Read for Decompressor<R> {

    fn read(&mut self, buf: &mut [u8]) -> Result<usize,io::Error> {
        loop {
            let r = match self.chunk {
                Some(ChunkType::CompressedData) => try!((&self.buf_dec[self.position ..]).read(buf)),
                Some(ChunkType::RawData) => try!((&self.buf[self.position ..]).read(buf)),
                _ => 0
            };
            if r>0 {
                self.position += r;
                return Ok(r);
            }
            self.chunk = None;
            try!(self.load_chunk());
            if self.chunk.is_none() {
                return Ok(0);
            }
        }
    }
}

#[test]
fn it_works() {
    let mut dec = Decompressor::new(
            fs::File::open("machin.snz").unwrap()
        );
    io::copy(&mut dec, &mut io::stdout()).unwrap();
}
