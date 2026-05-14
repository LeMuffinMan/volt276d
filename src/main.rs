mod hid;
mod protocol;
mod uinput;

fn main() {
    let mut device =
        uinput::create_uinput_device("/dev/uinput").expect("Error creating uinput device");
    hid::read_reports("/dev/hidraw0", &mut device.file).expect("Error reading HID device");
}
