

pub fn read_u16(data: &[u8], offset: usize) -> u16 {
    (data[offset] as u16) | ( (data[offset + 1] as u16) << 8)
}

pub fn read_u32(data: &[u8], offset: usize) -> u32 {
    let lo = read_u16(data, offset) as u32;
    let hi = read_u16(data, offset + 2) as u32;
    lo | (hi << 16)
}
