use std::fs::File;
use std::io::{self, Read};

use crate::protocol::{parse_compressor, parse_vintage};
use crate::uinput::{emit_event, ABS_X, ABS_Y, EV_ABS, EV_SW, SW_VINTAGE_1, SW_VINTAGE_2};

pub fn read_reports(path: &str, uinput: &mut File) -> io::Result<()> {
    let mut file = File::open(path)?;

    let mut buf = [0u8; 64];
    loop {
        let n = file.read(&mut buf)?;
        println!("buf = {:02x?}", &buf[0..n]);

        // ignore all reports except Report 0x02 (control state stream)
        if buf[0] != 0x02 {
            continue;
        }

        let (v1, v2) = parse_vintage(buf[1]);
        if let Err(e) = emit_event(uinput, EV_SW as u16, SW_VINTAGE_1 as u16, v1) {
            eprintln!("Error emitting vintage 1: {e}");
        }
        if let Err(e) = emit_event(uinput, EV_SW as u16, SW_VINTAGE_2 as u16, v2) {
            eprintln!("Error emitting vintage 2: {e}");
        }

        if let Some(mode) = parse_compressor(buf[2]) {
            println!("Compressor 1: {mode}");
            if let Err(e) = emit_event(uinput, EV_ABS as u16, ABS_X as u16, mode) {
                eprintln!("Error emitting compressor 1: {e}");
            }
        }
        if let Some(mode) = parse_compressor(buf[3]) {
            println!("Compressor 2: {mode}");
            if let Err(e) = emit_event(uinput, EV_ABS as u16, ABS_Y as u16, mode) {
                eprintln!("Error emitting compressor 2: {e}");
            }
        }
    }
}
