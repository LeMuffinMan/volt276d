// Volt 276 HID protocol — Report 0x02, continuous stream, 14 bytes
// byte[1] = flags (bit 0x40 = vintage input 1, bit 0x80 = vintage input 2)
// byte[2] = compressor input 1, byte[3] = compressor input 2

pub fn parse_compressor(byte: u8) -> Option<i32> {
    match byte {
        0x02 => Some(0), // Off
        0x11 => Some(1), // Voc
        0x09 => Some(2), // GTR
        0x05 => Some(3), // Fast
        _ => None,
    }
}

// Returns (vintage_input1, vintage_input2) as 0 or 1
pub fn parse_vintage(byte: u8) -> (i32, i32) {
    (
        if byte & 0x40 != 0 { 1 } else { 0 },
        if byte & 0x80 != 0 { 1 } else { 0 },
    )
}
