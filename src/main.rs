use std::fs::File;
use std::io;
use std::io::Read;

fn parse_compressor(byte: u8) -> Option<String> {
    match byte {
        0x02 => Some("Off".into()),
        0x11 => Some("Voc".into()),
        0x09 => Some("GTR".into()),
        0x05 => Some("Fast".into()),
        _ => None,
    }
}

fn read_reports(path: &str) -> io::Result<()> {
    let mut file = File::open(path)?;

    let mut buf = [0u8; 64];
    loop {
        let _n = file.read(&mut buf)?;
        // println!("Buff = {:02x?}", &buf[..n]);
        println!("");
        if let Some(mode) = parse_compressor(buf[2]) {
            println!("Compressor 1: {mode}");
        }
        if let Some(mode) = parse_compressor(buf[3]) {
            println!("Compressor 2: {mode}");
        }
    }
}

fn main() {
    read_reports("/dev/hidraw0").expect("Error reading path")
}
