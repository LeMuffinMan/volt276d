use std::fs::File;
use std::io::{self, Read};

use crate::protocol::parse_compressor;
use crate::uinput::{ABS_X, ABS_Y, EV_ABS, emit_event};

pub fn read_reports(path: &str, uinput: &mut File) -> io::Result<()> {
    let mut file = File::open(path)?;

    let mut buf = [0u8; 64];
    loop {
        let n = file.read(&mut buf)?;
        println!("buf = {:02x?}", &buf[0..n]);
        if let Some(mode) = parse_compressor(buf[2]) {
            println!("Compressor 1: {mode}");
            if let Err(e) = emit_event(uinput, EV_ABS as u16, ABS_X as u16, mode) {
                eprintln!("Error emitting event: {e}");
            }
        }
        if let Some(mode) = parse_compressor(buf[3]) {
            println!("Compressor 2: {mode}");
            if let Err(e) = emit_event(uinput, EV_ABS as u16, ABS_Y as u16, mode) {
                eprintln!("Error emitting event: {e}");
            }
        }
    }
}
