
use object::{File as ObjFile, CoffFile, Object};
use std::fs::File;
use std::io::{Read, empty};
use gimli::{SectionId, Error, ReaderOffsetId, EndianReader, LittleEndian};
use std::borrow::Cow;
use std::sync::Arc;
use std::ops::Deref;
use gimli::Location::Bytes;

#[derive(Clone, Debug)]
struct ByteVec(Arc<Vec<u8>>);

impl ByteVec {
    fn new() -> Self {
        ByteVec(Arc::new(Vec::new()))
    }
}

impl From<Vec<u8>> for ByteVec {
    fn from(data: Vec<u8>) -> Self {
        ByteVec(Arc::new(data))
    }
}

impl Deref for ByteVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl gimli::StableDeref for ByteVec {}
unsafe impl gimli::CloneStableDeref for ByteVec {}


type Reader = EndianReader<LittleEndian, ByteVec>;
type Dwarf = gimli::Dwarf<Reader>;
type GimliError = Result<Reader, gimli::Error>;

fn empty_reader() -> Reader {
    Reader::new(ByteVec::new(), LittleEndian::default())
}

fn get_section_data<'data, 'file : 'data, T: Object<'data, 'file>>(obj: &'file T, id: SectionId) -> Result<Reader, &'static str> {
    Ok(obj.section_data_by_name(id.name())
        .map(|x| {
            let x = x.iter().map(|x| *x).collect::<Vec<u8>>();
            EndianReader::new(x.into(), LittleEndian::default())
        })
        .unwrap_or_else(empty_reader)
    )
}

fn main() {
    let mut file = File::open("f1_btl.out").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let obj = CoffFile::parse(&data).unwrap();

    let dwarf = Dwarf::load(
        |id| get_section_data(&obj, id),
            |id| {
        Ok(empty_reader())
    }).expect("Cannot find dwarf section in file");

    let mut iter = dwarf.units();

    while let Some(unit) = iter.next().unwrap() {
        println!("unit's length is {}", unit.unit_length());
    }

    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {

    }
}