use crc;
use snappy;

use std::io;
use std::io::prelude::*;
use std::fmt;

use byteorder::{ ByteOrder, ReadBytesExt, WriteBytesExt, LittleEndian };

#[derive(Debug)]
enum ChunkType {
    StreamIdentifier,
    CompressedData,
    RawData,
    Padding,
    ReservedUnskippable,
    ReservedSkippable,
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

    #[allow(dead_code)]
    pub fn new(r:R) -> Decompressor<R> {
        Decompressor { inner:r, buf: Vec::new(), buf_dec:Vec::with_capacity(65536),
            chunk:None, check_crc: true, check_stream_identifier: true,
            position: 0
        }
    }

    #[allow(dead_code)]
    pub fn check_crc(self, v:bool) -> Decompressor<R> {
        Decompressor { check_crc:v, .. self }
    }

    #[allow(dead_code)]
    pub fn check_stream_identifier(self, v:bool) -> Decompressor<R> {
        Decompressor { check_stream_identifier:v, .. self }
    }

    #[allow(dead_code)]
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
            try!(snappy::uncompress(&self.buf[4 ..], &mut self.buf_dec));
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
        } else if kind[0] == 0xfe {
            self.chunk = Some(ChunkType::Padding);
        } else if kind[0] >= 0x02 && kind[0] <= 0x7f {
            self.chunk = Some(ChunkType::ReservedUnskippable);
            return Err(io::Error::new(io::ErrorKind::Other,
                "Reserved unskippable chunk. Cowardly bailing out."));
        } else if kind[0] >= 0x80 && kind[0] <= 0xfd {
            self.chunk = Some(ChunkType::ReservedSkippable);
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

pub struct Compressor<W : Write> {
    inner:W,
    header_sent:bool,
    buf:Vec<u8>,
    buf_dec:Vec<u8>,
}

impl<W : Write> Compressor<W> {

    #[allow(dead_code)]
    pub fn new(w:W) -> Compressor<W> {
        Compressor { inner:w, buf: Vec::new(),
            buf_dec: Vec::with_capacity(65536),
            header_sent:false }
    }

    fn compress_and_write(&mut self) -> io::Result<()> {
        if !self.header_sent {
            try!( self.inner.write_all(
                &[  0xff, 0x06, 0x00, 0x00, 0x73,
                    0x4e, 0x61, 0x50, 0x70, 0x59 ]
            ));
            self.header_sent = true;
        }
        try!(snappy::compress(&self.buf_dec, &mut self.buf));
        let crc:u32 = crc::crc32::checksum_castagnoli(&self.buf_dec);
        let sum = ((crc >> 15) | (crc << 17)).wrapping_add(0xa282ead8);
        try!(self.inner.write_u32::<LittleEndian>(((4+self.buf.len()) as u32) << 8));
        try!(self.inner.write_u32::<LittleEndian>(sum));
        self.buf_dec.clear();
        self.inner.write_all(&self.buf)
    }
}

impl<W : Write> Write for Compressor<W> {

    fn write(&mut self, buf:&[u8]) -> io::Result<usize> {
        let w = try!(self.buf_dec.write(buf));
        if self.buf_dec.len() == self.buf_dec.capacity() {
            try!(self.compress_and_write());
        }
        Ok(w)
    }

    fn flush(&mut self) -> io::Result<()> {
        try!(self.compress_and_write());
        self.inner.flush()
    }
}

impl<W : Write> Drop for Compressor<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
