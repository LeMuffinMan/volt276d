// Volt 276 HID protocol — Report 0x02, continuous stream
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
