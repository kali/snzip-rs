extern crate byteorder;
extern crate snappy;

use std::io;
use std::io::prelude::*;

use std::fmt;

use byteorder::{ ReadBytesExt, LittleEndian };

#[derive(Debug)]
enum ChunkReader {
    StreamIdentifier,
    CompressedData(io::Cursor<Vec<u8>>)
}

impl Read for ChunkReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize,io::Error> {
        match *self {
            ChunkReader::StreamIdentifier => Ok(0),
            ChunkReader::CompressedData(ref mut cursor) => cursor.read(buf),
        }
    }
}

pub struct Decompressor<R : Read> {
    inner:R,
    buf:Vec<u8>,
    chunk:Option<ChunkReader>
}

impl<R : Read> fmt::Debug for Decompressor<R> {

    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decompressor: {:?} {:?}", &self.buf.len(), &self.chunk)
    }

}

impl<R : Read> Decompressor<R> {

    pub fn new(r:R) -> Decompressor<R> {
        Decompressor { inner:r, buf: Vec::new(), chunk:None }
    }

    fn load_chunk(&mut self) -> Result<Option<ChunkReader>, io::Error> {
        let mut kind:[u8;1] = [0];
        if try!(self.inner.read(&mut kind)) == 0 {
            return Ok(None)
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
            Ok(Some(ChunkReader::StreamIdentifier))
        } else if kind[0] == 0x00 {
            let page:Vec<u8> = try!(snappy::uncompress(&self.buf[4 ..]).ok_or(
                io::Error::new(io::ErrorKind::Other, "incomplete page")
            ));
            Ok(Some(ChunkReader::CompressedData(io::Cursor::new(page))))
        } else {
            Ok(Some(ChunkReader::StreamIdentifier))
        }
    }
}

impl<R : Read> Read for Decompressor<R> {

    fn read(&mut self, buf: &mut [u8]) -> Result<usize,io::Error> {
        loop {
            let r = match self.chunk {
                Some(ref mut c) => try!(c.read(buf)),
                None => 0
            };
            if r>0 {
                return Ok(r);
            }
            self.chunk = try!(self.load_chunk());
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
