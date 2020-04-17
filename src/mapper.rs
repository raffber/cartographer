//! This module supports parsing DWARF sections
//! of an executable and exposes the relevant information
//! for producing a map file.

use gimli::constants::{DW_AT_name, DW_AT_type, DW_TAG_member, DW_TAG_typedef, DW_AT_location,
                       DW_TAG_structure_type, DW_AT_data_member_location, DW_TAG_variable, DW_TAG_base_type};
use gimli::{AttributeValue, Encoding, Location, CompilationUnitHeader};
use crate::Reader;
use std::collections::HashMap;
use gimli::EvaluationResult::RequiresRelocatedAddress;


#[derive(Debug, Clone)]
pub struct Structure {
    pub name: Option<String>,
    pub type_offset: usize,
    pub members: Vec<StructMember>,
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub name: String,
    pub type_offset: usize,
    pub member_offset: usize,
    pub fields: Vec<StructMember>,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub address: u64,
    pub name: String,
    pub type_offset: usize,
    pub fields: Vec<StructMember>,
}

#[derive(Debug, Clone)]
pub struct Typedef {
    pub name: String,
    pub type_offset: usize,
}

pub struct Mapper {
    pub encoding: Encoding,
    pub typedefs: HashMap<usize, Typedef>,
    pub structs: HashMap<usize, Structure>,
    pub globals: Vec<Variable>,
    pub base_types: HashMap<usize, String>,
}

impl Mapper {
    pub fn new(encoding: Encoding) -> Mapper {
        Mapper {
            encoding,
            typedefs: HashMap::new(),
            structs: HashMap::new(),
            globals: vec![],
            base_types: Default::default()
        }
    }

    pub fn process_tree(&mut self, node: gimli::EntriesTreeNode<Reader>, level: u32, unit: &CompilationUnitHeader<Reader>) -> gimli::Result<()> {
        match node.entry().tag() {
            DW_TAG_structure_type => self.process_struct(node, unit),
            DW_TAG_typedef => self.process_typedef(node, unit),
            DW_TAG_variable => self.process_variable(node, level),
            DW_TAG_base_type => self.process_type(node, unit),
            _ => {
                let mut children = node.children();
                while let Some(child) = children.next()? {
                    self.process_tree(child, level+1, unit)?;
                }
                Ok(())
            }
        }
    }

    pub fn postprocess(&mut self) {
        for (addr, td) in &self.typedefs {
            if let Some(strct) = self.structs.get_mut(&td.type_offset) {
                strct.name = Some(td.name.clone());
                let strct = strct.clone();
                self.structs.insert(*addr, strct);
            }

            if let Some(base_type) = self.base_types.get(&td.type_offset) {
                let base_type = base_type.clone();
                self.base_types.insert(*addr, base_type);
            }
        }

        let mut new_strcts = HashMap::new();
        let addrs: Vec<_> = self.structs.keys().map(|x| *x).collect();
        for addr in &addrs {
            self.build_struct(&mut new_strcts, *addr);
        }
        self.structs = new_strcts;

        for global in &mut self.globals {
            if let Some(x) = self.structs.get(&global.type_offset) {
                global.fields = x.members.clone();
            }
        }
    }

    pub fn resolve_struct(&self, offset: usize) ->  Option<Structure> {
        self.structs.get(&offset).map(|x| x.clone())
    }

    fn build_struct(&mut self, new_strcts: &mut HashMap<usize, Structure>, strct_addr: usize) -> Vec<StructMember> {
        let mut ret = Vec::new();
        let mut strct = self.structs.get(&strct_addr).unwrap().clone();
        for member in &strct.members {
            ret.push(member.clone());
        }

        for member in &mut ret {
            if self.structs.contains_key(&member.type_offset) {
                member.fields = self.build_struct(new_strcts, member.type_offset);
            }
        }

        strct.members = ret.clone();
        new_strcts.insert(strct_addr, strct);
        ret
    }

    fn process_type(&mut self, node: gimli::EntriesTreeNode<Reader>, unit: &CompilationUnitHeader<Reader>) -> gimli::Result<()> {
        let type_offset = node.entry().offset().to_debug_info_offset(unit).0;
        let name = if let Some(AttributeValue::String(name)) = node.entry().attr_value(DW_AT_name)? {
            std::str::from_utf8(&name).unwrap().to_string()
        } else {
            return Ok(());
        };
        self.base_types.insert(type_offset, name);
        Ok(())
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
            member_offset,
            fields: vec![]
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

    fn process_struct(&mut self, node: gimli::EntriesTreeNode<Reader>, unit: &CompilationUnitHeader<Reader>) -> gimli::Result<()> {
        let name = if let Some(name) = node.entry().attr_value(DW_AT_name)? {
            if let AttributeValue::String(name) = name {
                Some(std::str::from_utf8(&name).unwrap().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let offset = node.entry().offset().to_debug_info_offset(unit).0;
        let members = self.process_struct_members(node)?;

        self.structs.insert(offset, Structure {
            name,
            type_offset: offset,
            members
        });

        Ok(())
    }

    fn process_typedef(&mut self, node: gimli::EntriesTreeNode<Reader>, unit: &CompilationUnitHeader<Reader>) -> gimli::Result<()> {
        if let Some(name) = node.entry().attr_value(DW_AT_name)? {
            let name = if let AttributeValue::String(name) = name {
                std::str::from_utf8(&name).unwrap().to_string()
            } else {
                return Ok(());
            };
            if let Some(AttributeValue::DebugInfoRef(offset)) = node.entry().attr_value(DW_AT_type)? {
                let td_offset = node.entry().offset().to_debug_info_offset(unit).0;
                let td = Typedef {
                    name,
                    type_offset: offset.0
                };

                self.typedefs.insert(td_offset, td);
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
            address: location,
            name,
            type_offset: type_offset.0,
            fields: vec![]
        });
        Ok(())
    }
}