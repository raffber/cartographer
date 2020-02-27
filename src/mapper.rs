use gimli::constants::{DW_AT_name, DW_AT_type, DW_TAG_member, DW_TAG_typedef, DW_AT_location,
                       DW_TAG_structure_type, DW_AT_data_member_location, DW_TAG_variable};
use gimli::{AttributeValue, UnitOffset, Encoding, Location};
use crate::Reader;
use std::collections::HashMap;
use gimli::EvaluationResult::RequiresRelocatedAddress;


pub struct Structure {
    pub name: Option<String>,
    pub offset: UnitOffset<usize>,
    pub members: Vec<StructMember>,
}

pub struct StructMember {
    pub name: String,
    pub type_offset: usize,
    pub member_offset: usize,
}

pub struct Variable {
    pub location: u64,
    pub name: String,
    pub type_offset: usize,
}

pub struct Typedef {
    pub name: String,
    pub type_offset: usize,
}

pub struct Mapper {
    pub encoding: Encoding,
    pub typedefs: HashMap<usize, Typedef>,
    pub structs: HashMap<usize, Structure>,
    pub globals: Vec<Variable>,
}

impl Mapper {
    pub fn new(encoding: Encoding) -> Mapper {
        Mapper {
            encoding,
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            globals: vec![]
        }
    }

    pub fn resolve_struct(&self, offset: usize) -> Option<&Structure> {
        if let Some(td) = self.typedefs.get(&offset) {
            self.resolve_struct(td.type_offset)
        } else if let Some(strct) = self.structs.get(&offset) {
            Some(strct)
        } else {
            None
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
            type_offset: type_offset.0,
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

        self.structs.insert(offset.0, Structure {
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
                let td = Typedef {
                    name,
                    type_offset: offset.0
                };
                self.typedefs.insert(node.entry().offset().0, td);
            }
        }
        Ok(())
    }

    pub fn process_variable(&mut self, node: gimli::EntriesTreeNode<Reader>, level: u32) -> gimli::Result<()> {
        if level > 1 {
            return Ok(());
        }

        let name = if let Some(AttributeValue::String(name)) = node.entry().attr_value(DW_AT_name)? {
            std::str::from_utf8(&name).unwrap().to_string()
        } else {
            return Ok(());
        };

        let type_offset = if let Some(AttributeValue::DebugInfoRef(offset)) = node.entry().attr_value(DW_AT_type)? {
            offset
        } else {
            return Ok(());
        };

        let location = if let Some(AttributeValue::Exprloc(expr)) = node.entry().attr_value(DW_AT_location)? {
            let mut evaluation = expr.evaluation(self.encoding.clone());
            if let RequiresRelocatedAddress(addr) = evaluation.evaluate().unwrap() {
                addr
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        };

        self.globals.push(Variable {
            location,
            name,
            type_offset: type_offset.0
        });
        Ok(())
    }

    pub fn process_tree(&mut self, node: gimli::EntriesTreeNode<Reader>, level: u32) -> gimli::Result<()> {
        match node.entry().tag() {
            DW_TAG_structure_type => self.process_struct(node),
            DW_TAG_typedef => self.process_typedef(node),
            DW_TAG_variable => self.process_variable(node, level),
            _ => {
                let mut children = node.children();
                while let Some(child) = children.next()? {
                    self.process_tree(child, level+1)?;
                }
                Ok(())
            }
        }
    }
}