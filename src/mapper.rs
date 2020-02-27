use gimli::constants::{DW_AT_name, DW_AT_type, DW_TAG_member, DW_TAG_typedef, DW_TAG_structure_type, DW_AT_data_member_location};
use gimli::{AttributeValue, DebugInfoOffset, UnitOffset, Encoding, Location};
use crate::Reader;


pub struct Structure {
    name: Option<String>,
    offset: UnitOffset<usize>,
    members: Vec<StructMember>,
}

pub struct StructMember {
    name: String,
    type_offset: DebugInfoOffset<usize>,
    member_offset: usize,
}

pub struct Mapper {
    pub encoding: Encoding,
    pub typedefs: Vec<(String, DebugInfoOffset<usize>)>,
    pub structs: Vec<Structure>,
}

impl Mapper {
    pub fn new(encoding: Encoding) -> Mapper {
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

    pub fn process_tree(&mut self, node: gimli::EntriesTreeNode<Reader>) -> gimli::Result<()> {
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