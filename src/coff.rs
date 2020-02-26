use crate::parse::{read_u32, read_u16};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub struct CoffFile<'data> {
    data: &'data [u8],
    header: Header<'data>,
    section_headers: SectionHeaders<'data>,
}

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

    fn parse(data: &[u8], num_sections: usize) -> SectionHeaders {
        assert_eq!(data.len(), num_sections * CoffFile::SECTION_HEADER_LENGTH);
        let mut headers = Vec::new();
        for k in 0..num_sections {
            let header_data = &data[k*CoffFile::SECTION_HEADER_LENGTH..(k + 1)*CoffFile::SECTION_HEADER_LENGTH];
            headers.push(SectionHeader::parse(header_data))
        }
        SectionHeaders {
            data,
            headers
        }
    }

}

#[derive(Clone)]
struct SectionHeader<'data> {
    data: &'data [u8]
}

impl<'data> SectionHeader<'data> {
    fn parse(data: &'data [u8]) -> SectionHeader {
        assert_eq!(data.len(), CoffFile::SECTION_HEADER_LENGTH);
        println!("{:?}", &data[0..8]);
        SectionHeader {
            data
        }
    }
}


impl<'data> CoffFile<'data> {

    const HEADER_LENGTH: usize = 22;
    const SECTION_HEADER_LENGTH: usize = 48;

    pub fn parse(data: &'data [u8]) -> Result<Self> {
        let header = CoffFile::parse_header(&data)?;
        let section_headers_start_addr = (header.optional_header_size() as usize) + CoffFile::HEADER_LENGTH;
        let section_headers_end_addr = section_headers_start_addr + header.number_of_sections() * CoffFile::SECTION_HEADER_LENGTH;
        let section_header_data = &data[section_headers_start_addr..section_headers_end_addr];
        let section_headers = SectionHeaders::parse(section_header_data, header.number_of_sections());

        Ok(CoffFile {
            data,
            header,
            section_headers
        })
    }

    fn parse_header(data: &[u8]) -> Result<Header> {
        Ok(Header {
            data
        })
    }

    pub fn header(&self) -> Header {
        self.header.clone()
    }
}
