#![allow(non_upper_case_globals)]
#![allow(dead_code)]


use std::fs::File;
use std::io::Read;
use gimli::{SectionId, EndianReader, LittleEndian, AttributeValue, DebugInfoOffset, UnitOffset, Encoding, Location};
use std::sync::Arc;
use std::ops::Deref;
use crate::coff::CoffFile;
use gimli::constants::{DW_AT_name, DW_AT_type, DW_TAG_member, DW_TAG_typedef, DW_TAG_structure_type, DW_AT_data_member_location};

mod coff;
mod parse;

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

struct Structure {
    name: Option<String>,
    offset: UnitOffset<usize>,
    members: Vec<StructMember>,
}

struct StructMember {
    name: String,
    type_offset: DebugInfoOffset<usize>,
    member_offset: usize,
}

struct Mapper {
    encoding: Encoding,
    typedefs: Vec<(String, DebugInfoOffset<usize>)>,
    structs: Vec<Structure>,
}

impl Mapper {
    fn new(encoding: Encoding) -> Mapper {
        Mapper {
            encoding,
            typedefs: vec![],
            structs: vec![],
        }
    }

    fn process_struct_member(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<Option<StructMember>> {
        let name = if let Some(AttributeValue::String(name)) = node.entry().attr_value(DW_AT_name)? {
             std::str::from_utf8(&name).unwrap().to_string()
        } else {
            return Ok(None);
        };

        let type_offset = if let Some(AttributeValue::DebugInfoRef(offset)) = node.entry().attr_value(DW_AT_type)? {
            offset
        } else {
            return Ok(None);
        };

        let member_offset = if let Some(AttributeValue::Exprloc(expr)) = node.entry().attr_value(DW_AT_data_member_location)? {
            let mut evaluation = expr.evaluation(self.encoding.clone());
            evaluation.set_initial_value(0);
            evaluation.evaluate().unwrap();
            let result = evaluation.result();
            let result = &result[0];
            let result = match result.location {
                Location::Address { address } => {address},
                _ => return Ok(None)
            };
            result as usize
        } else {
            return Ok(None);
        };

        Ok(Some(StructMember {
            name,
            type_offset,
            member_offset
        }))
    }

    fn process_struct_members(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<Vec<StructMember>> {
        let mut ret = Vec::new();
        let mut children = node.children();
        while let Some(child) = children.next()? {
            if child.entry().tag() != DW_TAG_member {
                continue;
            }
            self.process_struct_member(child)?.map(|x| ret.push(x));
        }
        Ok(ret)
    }

    fn process_struct(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<()> {
        let name = if let Some(name) = node.entry().attr_value(DW_AT_name)? {
            if let AttributeValue::String(name) = name {
                Some(std::str::from_utf8(&name).unwrap().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let offset = node.entry().offset();
        let members = self.process_struct_members(node)?;

        self.structs.push(Structure {
            name,
            offset,
            members
        });

        Ok(())
    }

    fn process_typedef(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<()> {
        if let Some(name) = node.entry().attr_value(DW_AT_name)? {
            let name = if let AttributeValue::String(name) = name {
                std::str::from_utf8(&name).unwrap().to_string()
            } else {
                return Ok(());
            };
            if let Some(AttributeValue::DebugInfoRef(offset)) = node.entry().attr_value(DW_AT_type)? {
                self.typedefs.push((name, offset));
            }
        }
        Ok(())
    }

    fn process_tree(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<()> {
        match node.entry().tag() {
            DW_TAG_structure_type => self.process_struct(node),
            DW_TAG_typedef => self.process_typedef(node),
            _ => {
                let mut children = node.children();
                while let Some(child) = children.next()? {
                    self.process_tree(child)?;
                }
                Ok(())
            }
        }
    }
}


fn main() {
    let mut file = File::open("f1_btl.out").unwrap();
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
        let _ = mapper.process_tree(root);
    }

    println!("Structs found: {}", mapper.structs.len());
    println!("Typedefs found: {}", mapper.typedefs.len());

    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {

    }
}