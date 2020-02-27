#![allow(non_upper_case_globals)]
#![allow(dead_code)]


use std::fs::File;
use std::io::Read;
use gimli::{SectionId, EndianReader, LittleEndian};
use std::sync::Arc;
use std::ops::Deref;
use crate::coff::CoffFile;
use crate::mapper::Mapper;
use crate::mapfile::Mapfile;

mod coff;
mod parse;
mod mapper;
mod mapfile;

#[derive(Clone, Debug)]
pub struct ByteVec(Arc<Vec<u8>>);

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

fn empty_reader() -> Reader {
    Reader::new(ByteVec::new(), LittleEndian::default())
}

fn get_section_data(obj: &CoffFile, id: SectionId) -> Result<Reader, &'static str> {
    let ret = obj
        .get_section(id.name())
        .map(|x| Reader::new(x.data().into(), LittleEndian::default()) )
        .unwrap_or_else(empty_reader);
    Ok(ret)
}

fn main() {
    let mut file = File::open("f1_app.out").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let obj = CoffFile::parse(&data).unwrap();

    let dwarf = Dwarf::load(
        |id| get_section_data(&obj, id),
            |_| Ok(empty_reader())
    ).expect("Cannot find dwarf section in file");

    let mut mapper = Mapper::new(dwarf.units().next().unwrap().unwrap().encoding());
    let mut iter = dwarf.units();
    while let Some(unit) = iter.next().unwrap() {
        let abbrev = dwarf.abbreviations(&unit).unwrap();
        let mut tree = unit.entries_tree(&abbrev, None).unwrap();
        let root = tree.root().unwrap();
        let _ = mapper.process_tree(root, 0);
    }

    println!("Structs found: {}", mapper.structs.len());
    println!("Typedefs found: {}", mapper.typedefs.len());
    println!("Globals found: {}", mapper.globals.len());

    let mapfile = Mapfile::new(mapper);

    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {

    }
}