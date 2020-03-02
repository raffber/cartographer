#![allow(non_upper_case_globals)]
#![allow(dead_code)]


use std::fs::File;
use std::io::{Read, Write};
use gimli::{SectionId, EndianReader, LittleEndian};
use std::sync::Arc;
use std::ops::Deref;
use crate::coff::CoffFile;
use crate::mapper::Mapper;
use crate::mapfile::Mapfile;
use std::path::PathBuf;
use clap::{App, Arg};

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


fn produce_map(input_file: PathBuf, output_file: PathBuf, pretty: bool) {
    let mut file = File::open(input_file).expect("Cannot open input file");
    let mut data = Vec::new();
    file.read_to_end(&mut data).expect("Cannot read from output file");
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
        let _ = mapper.process_tree(root, 0, &unit);
    }
    mapper.postprocess();

    let mapfile = Mapfile::new(mapper);
    let serialized = if pretty {
        serde_json::to_string_pretty(&mapfile.entries).unwrap()
    } else {
        serde_json::to_string(&mapfile.entries).unwrap()
    };
    let mut outfile = File::create(output_file).expect("Cannot create output file");
    outfile.write_all(serialized.as_bytes()).expect("Cannot write to output file");
}

fn main() {
    let matches = App::new("cartographer - Produce map files like its 1999")
        .version("0.1")
        .author("Raphael Bernhard <beraphae@gmail.com>")
        .about("Extract DWARF information from executables and creates map files")
        .arg(Arg::with_name("input-file")
            .short("i")
            .long("input")
            .value_name("INPUT_FILE")
            .required(true)
            .help("Input file binary file to be processed."))
        .arg(Arg::with_name("output-file")
            .short("o")
            .long("output")
            .value_name("OUTPUT_FILE")
            .help("Output map files to be written."))
        .arg(Arg::with_name("pretty")
            .short("p")
            .long("pretty")
            .help("Defines whether the resulting json file should be pretty printed."))
        .get_matches();

    let input_file = matches.value_of("input-file").expect("No input file given");
    let output_file = matches.value_of("output-file")
        .map(|x| x.to_string())
        .unwrap_or_else(|| {
            let mut ret = input_file.to_string();
            ret.push_str(".json");
            ret
        });
    let pretty = matches.is_present("pretty");

    produce_map(input_file.into(), output_file.into(), pretty);
}
