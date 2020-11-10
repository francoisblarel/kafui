use byteorder::{BigEndian, ReadBytesExt};
use std::io::Error;
use std::io::{BufRead, Cursor};
use std::str;

pub fn read_str<'a>(rdr: &'a mut Cursor<&[u8]>) -> Result<&'a str, Error> {
    let len = (rdr.read_i16::<BigEndian>())? as usize;
    let pos = rdr.position() as usize;
    let slice = str::from_utf8(&rdr.get_ref()[pos..(pos + len)]).expect("kaboom");
    rdr.consume(len);
    Ok(slice)
}
