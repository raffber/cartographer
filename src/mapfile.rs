use crate::mapper::{Mapper, StructMember};
use gimli::Operation::EntryValue;

use serde::{Deserialize, Serialize};

pub struct Mapfile {
    pub entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<u64>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<Entry>,

    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    typ: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<usize>,
}

impl Entry {
    fn new() -> Self {
        Entry {
            addr: None,
            fields: vec![],
            name: None,
            typ: None,
            offset: None
        }
    }
}

impl Mapfile {
    pub fn new(mapper: Mapper) -> Mapfile {
        let mut entries = Vec::new();

        for global in &mapper.globals {
            let mut entry = Entry::new();
            entry.name = Some(global.name.clone());
            entry.addr = Some(global.address);

            if let Some(strct) = mapper.resolve_struct(global.type_offset) {
                strct.name.as_ref().map(|name| println!("{}", name) );
                let mut members = Vec::new();
                for member in &strct.members {
                    members.push(Self::member_to_entry(member));
                }
                entry.fields = members;
            }
            entries.push(entry);
        }

        Mapfile { entries }
    }


    fn member_to_entry(member: &StructMember) -> Entry {
        let fields = member.fields.iter()
            .map(|x| Self::member_to_entry(x))
            .collect();

        Entry {
            addr: None,
            fields,
            name: Some(member.name.clone()),
            typ: None,
            offset: Some(member.member_offset),
        }
    }
}