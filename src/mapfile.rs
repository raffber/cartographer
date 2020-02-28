use crate::mapper::Mapper;

pub struct Mapfile {

}

impl Mapfile {
    pub fn new(mapper: Mapper) -> Mapfile {
        for global in &mapper.globals {
            if &global.name == "sysctrl" {
                println!("0x{:X}", global.type_offset);
            }
            // println!("{}", global.name);
            if let Some(strct) = mapper.resolve_struct(global.type_offset) {
                strct.name.as_ref().map(|name| println!("{}", name) );
            }
        }

        // println!("{:?}", mapper.globals);

        Mapfile {

        }
    }
}