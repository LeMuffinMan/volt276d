use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;

// Source: <linux/input.h>
pub const EV_ABS: libc::c_int = 0x03; // event type "absolute axis"
pub const EV_SW: libc::c_int = 0x05;  // event type "switch"
pub const ABS_X: libc::c_int = 0x00; // axis X → compressor input 1
pub const ABS_Y: libc::c_int = 0x01; // axis Y → compressor input 2
pub const SW_VINTAGE_1: libc::c_int = 0x00; // vintage input 1 (SW slot 0)
pub const SW_VINTAGE_2: libc::c_int = 0x01; // vintage input 2 (SW slot 1)

// Source: <linux/uinput.h> — ioctl numbers calculated with _IOW('U', n, type)
// _IOW('U'=0x55, 100=0x64, int=4)             → 0x40045564
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
// _IOW('U'=0x55, 103=0x67, int=4)             → 0x40045567
const UI_SET_ABSBIT: libc::c_ulong = 0x40045567;
// _IO('U'=0x55, 1)  — no data                 → 0x5501
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
// _IO('U'=0x55, 2)  — no data                 → 0x5502
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
// _IOW('U'=0x55, 3, uinput_setup=92=0x5c)     → 0x405c5503
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503;
// _IOW('U'=0x55, 4, uinput_abs_setup=28=0x1c) → 0x401c5504
const UI_ABS_SETUP: libc::c_ulong = 0x401c5504;
// _IOW('U'=0x55, 109=0x6d, int=4)             → 0x4004556d
const UI_SET_SWBIT: libc::c_ulong = 0x4004556d;

// repr(C) ensures field layout matches the C struct — required for kernel ioctls
// kernel needs device identity, name, and force-feedback count (unused here)
#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

// USB device identity — matches VID:PID from lsusb
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
    kind: u16,           // event type: EV_ABS (0x03) or EV_SYN (0x00)
    code: u16,           // axis: ABS_X (compressor 1) or ABS_Y (compressor 2)
    value: i32,          // value for axis: 0=Off, 1=Voc, 2=GTR, 3=Fast
}

// RAII wrapper — uinput device is destroyed automatically when this struct is dropped
pub struct UinputDevice {
    pub file: File,
}

impl Drop for UinputDevice {
    fn drop(&mut self) {
        unsafe { libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY) };
    }
}

pub fn create_uinput_device(path: &str) -> io::Result<UinputDevice> {
    let file = OpenOptions::new().write(true).open(path)?;

    // EV_ABS for our 3-state compressor button
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_EVBIT, EV_ABS) };

    // two compressors
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_ABSBIT, ABS_X) };
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_ABSBIT, ABS_Y) };

    // EV_SW for vintage on/off toggles (no range setup needed for switches)
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_EVBIT, EV_SW) };
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_SWBIT, SW_VINTAGE_1) };
    unsafe { libc::ioctl(file.as_raw_fd(), UI_SET_SWBIT, SW_VINTAGE_2) };

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
    Ok(UinputDevice { file })
}

pub fn emit_event(uinput: &mut File, kind: u16, code: u16, value: i32) -> io::Result<()> {
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

    // EV_SYN / SYN_REPORT — signals the kernel that the event report is complete
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
