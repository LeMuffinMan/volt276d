use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::unix::io::AsRawFd;

// Source: <linux/input.h>
const EV_ABS: libc::c_int = 0x03; // event type "absolute axis"
const ABS_X: libc::c_int = 0x00; // axe X → compressorInput 1
const ABS_Y: libc::c_int = 0x01; // axe Y → compressor Input 2

// Source: <linux/uinput.h> — ioctl numbers calculated with _IOW('U', n, type)
// _IOW('U'=0x55, 100=0x64, int=4)           → 0x40045564
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
// _IOW('U'=0x55, 103=0x67, int=4)           → 0x40045567
const UI_SET_ABSBIT: libc::c_ulong = 0x40045567;
// _IO('U'=0x55, 1)  — pas de données        → 0x5501
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
// _IOW('U'=0x55, 3, uinput_setup=92=0x5c)   → 0x405c5503
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503;
// _IOW('U'=0x55, 4, uinput_abs_setup=28=0x1c) → 0x401c5504
const UI_ABS_SETUP: libc::c_ulong = 0x401c5504;

// Rust should not reorganise struct fields in memory, and reproduce the C way to do it : repr(C)
// the kernel need infos of the card, the name of my uinput device, force feedback (no need here)
#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

// describe my volt276 card
#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

// Source: <linux/input.h> — describes an absolute axis range
#[repr(C)]
struct InputAbsinfo {
    value: i32,      // current value (ignored at setup)
    minimum: i32,    // min value
    maximum: i32,    // max value
    fuzz: i32,       // noise filter — 0 = disabled
    flat: i32,       // dead zone — 0 = disabled
    resolution: i32, // units/mm — 0 = unknown
}

// Source: <linux/uinput.h> — axis declaration sent via UI_ABS_SETUP
#[repr(C)]
struct UinputAbsSetup {
    code: u16, // which axis (ABS_X, ABS_Y...)
    _pad: u16, // padding required by C struct alignment
    absinfo: InputAbsinfo,
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval, // timestamp — kernel will override
    kind: u16,           // type d'event : EV_ABS (0x03) or EV_SYN (0x00)
    code: u16,           // axis : ABS_X (compressor 1) ou ABS_Y (compressor 2)
    value: i32,          // value for axis : 0=Off, 1=Voc, 2=GTR, 3=Fast
}

fn parse_compressor(byte: u8) -> Option<i32> {
    match byte {
        0x02 => Some(0), // Off
        0x11 => Some(1), // Voc
        0x09 => Some(2), // GTR
        0x05 => Some(3), // Fast
        _ => None,
    }
}

fn read_reports(path: &str, uinput: &mut File) -> io::Result<()> {
    let mut file = File::open(path)?;

    let mut buf = [0u8; 64];
    loop {
        let _n = file.read(&mut buf)?;
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

fn create_uinput_device(path: &str) -> io::Result<File> {
    let file = OpenOptions::new().write(true).open(path)?;

    // déclarer EV_ABS comme type d'event supporté
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_EVBIT, EV_ABS) };

    // déclarer les deux axes
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_ABSBIT, ABS_X) };
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_ABSBIT, ABS_Y) };

    // configurer la plage de chaque axe : 0 (Off) → 3 (Fast)
    for code in [ABS_X as u16, ABS_Y as u16] {
        let abs_setup = UinputAbsSetup {
            code,
            _pad: 0,
            absinfo: InputAbsinfo {
                value: 0,
                minimum: 0,
                maximum: 3,
                fuzz: 0,
                flat: 0,
                resolution: 0,
            },
        };
        unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                UI_ABS_SETUP,
                &abs_setup as *const UinputAbsSetup,
            )
        };
    }

    let mut setup = UinputSetup {
        id: InputId {
            bustype: 0x03,   // BUS_USB → <linux/input.h>
            vendor: 0x2b5a,  // VID Volt 276 (lsusb)
            product: 0x0023, // PID Volt 276 (lsusb)
            version: 0x0001, // default version
        },
        name: [0; 80],
        ff_effects_max: 0,
    };
    let name = b"Volt 276 Controls";
    setup.name[..name.len()].copy_from_slice(name);
    unsafe { libc::ioctl(file.as_raw_fd(), UI_DEV_SETUP, &setup as *const UinputSetup) };
    unsafe { libc::ioctl(file.as_raw_fd(), UI_DEV_CREATE) };
    Ok(file)
}

fn emit_event(uinput: &mut File, kind: u16, code: u16, value: i32) -> io::Result<()> {
    let event = InputEvent {
        time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        kind,
        code,
        value,
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            &event as *const InputEvent as *const u8,
            std::mem::size_of::<InputEvent>(),
        )
    };
    uinput.write_all(bytes)?;

    // EV_SYN / SYN_REPORT — signal to kernel that report is complete
    let syn = InputEvent {
        time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        kind: 0x00,
        code: 0x00,
        value: 0,
    };
    let syn_bytes = unsafe {
        std::slice::from_raw_parts(
            &syn as *const InputEvent as *const u8,
            std::mem::size_of::<InputEvent>(),
        )
    };
    uinput.write_all(syn_bytes)
}

fn main() {
    let mut uinput_device =
        create_uinput_device("/dev/uinput").expect("Error creating uinput device");
    read_reports("/dev/hidraw0", &mut uinput_device).expect("Error reading path")
}
