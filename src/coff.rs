use crate::parse::{read_u32, read_u16};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub struct Header<'data> {
    data: &'data [u8],
}

impl<'data> Header<'data> {
    pub fn get_target_id(&self) -> u16 {
        (self.data[20] as u16) | ( (self.data[21] as u16) << 8)
    }

    pub fn number_of_sections(&self) -> usize {
        ((self.data[2] as u16) | ( (self.data[3] as u16) << 8)) as usize
    }

    pub fn symbol_table_start(&self) -> u32 {
        read_u32(self.data, 8)
    }

    pub fn symbol_table_size(&self) -> u32 {
        read_u32(self.data, 12)
    }

    pub fn optional_header_size(&self) -> u16 {
        read_u16(self.data, 16)
    }
}

#[derive(Clone)]
struct SectionHeaders<'data> {
    data: &'data [u8],
    headers: Vec<SectionHeader<'data>>
}

impl<'data> SectionHeaders<'data> {

    fn parse(data: &'data [u8], strings: &StringTable<'data> ,num_sections: usize) -> SectionHeaders<'data> {
        assert_eq!(data.len(), num_sections * CoffFile::SECTION_HEADER_LENGTH);
        let mut headers = Vec::new();
        for k in 0..num_sections {
            let header_data = &data[k*CoffFile::SECTION_HEADER_LENGTH..(k + 1)*CoffFile::SECTION_HEADER_LENGTH];
            headers.push(SectionHeader::parse(header_data, strings))
        }
        SectionHeaders {
            data,
            headers
        }
    }

}

#[derive(Clone)]
struct SectionHeader<'data> {
    data: &'data [u8],
    name: String,
}

impl<'data> SectionHeader<'data> {
    fn parse(data: &'data [u8], strings: &StringTable<'data>) -> SectionHeader<'data> {
        assert_eq!(data.len(), CoffFile::SECTION_HEADER_LENGTH);
        let name = strings.get_string(&data[0..8]).unwrap();
        SectionHeader {
            data,
            name
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn section_start_addr(&self) -> usize {
        read_u32(self.data, 20) as usize
    }

    pub fn section_length(&self) -> usize {
        read_u32(self.data, 16) as usize
    }
}

#[derive(Clone)]
pub struct Section<'data> {
    header: SectionHeader<'data>,
    data: &'data [u8],
}


impl<'data> Section<'data> {
    fn parse(data: &'data [u8], header: SectionHeader<'data>) -> Section<'data> {
        Section {
            data, header
        }
    }

    pub fn data(&self) -> Vec<u8> {
        self.data.to_vec()
    }
}

#[derive(Clone)]
struct SymbolTable<'data> {
    data: &'data [u8]
}

impl<'data> SymbolTable<'data> {
    fn parse(data: &'data [u8]) -> SymbolTable {
        SymbolTable {
            data
        }
    }
}

#[derive(Clone)]
struct StringTable<'data> {
    data: &'data [u8]
}

impl<'data> StringTable<'data> {
    fn parse(data: &'data [u8]) -> StringTable {
        let len = read_u32(data, 0) as usize;
        assert_eq!(len, data.len());
        StringTable {
            data
        }
    }

    fn get_string(&self, data: &[u8]) -> Option<String> {
        assert_eq!(data.len(), 8);
        let range = if data[0] == 0 {
            let string_ptr = read_u32(data, 4) as usize;
            &self.data[string_ptr..self.data.len()]
        } else {
            data
        };
        let non_zero: Vec<u8> = range.iter().map(|x| *x).take_while(|x| *x != 0).collect();
        std::str::from_utf8(&non_zero).ok().map(|x| x.into())
    }
}


#[derive(Clone)]
pub struct CoffFile<'data> {
    data: &'data [u8],
    header: Header<'data>,
    section_headers: SectionHeaders<'data>,
    sections: Vec<Section<'data>>,
    strings: StringTable<'data>,
    symbols: SymbolTable<'data>,
}


impl<'data> CoffFile<'data> {

    const HEADER_LENGTH: usize = 22;
    const SECTION_HEADER_LENGTH: usize = 48;
    const SYMBOL_LENGTH: usize = 18;

    pub fn parse(data: &'data [u8]) -> Result<Self> {
        let header = CoffFile::parse_header(&data)?;
        let section_headers_start_addr = (header.optional_header_size() as usize) + CoffFile::HEADER_LENGTH;
        let section_headers_end_addr = section_headers_start_addr + header.number_of_sections() * CoffFile::SECTION_HEADER_LENGTH;
        let section_header_data = &data[section_headers_start_addr..section_headers_end_addr];

        let symbol_table_start = header.symbol_table_start() as usize;
        let symbol_table_end = symbol_table_start + (header.symbol_table_size() as usize) * CoffFile::SYMBOL_LENGTH;
        let symbol_table_data = &data[symbol_table_start..symbol_table_end];

        let string_table_data = &data[symbol_table_end..data.len()];

        let string_table = StringTable::parse(string_table_data);
        let section_headers = SectionHeaders::parse(section_header_data, &string_table, header.number_of_sections());
        let symbol_table = SymbolTable::parse(symbol_table_data);

        let mut sections = Vec::new();
        for header in &section_headers.headers {
            let start_addr = header.section_start_addr();
            if start_addr == 0 || header.section_length() == 0 {
                continue;
            }
            let end_addr = start_addr + header.section_length();
            let raw_data = &data[start_addr..end_addr];
            let section = Section::parse(raw_data, header.clone());
            sections.push(section);
        }

        Ok(CoffFile {
            data,
            header,
            section_headers,
            sections,
            strings: string_table,
            symbols: symbol_table
        })
    }

    fn parse_header(data: &[u8]) -> Result<Header> {
        Ok(Header {
            data
        })
    }

    pub fn get_section(&self, name: &str) -> Option<Section<'data>> {
        for section in &self.sections {
            if &section.header.name == name {
                return Some(section.clone())
            }
        }
        None
    }

    pub fn header(&self) -> Header {
        self.header.clone()
    }
}
