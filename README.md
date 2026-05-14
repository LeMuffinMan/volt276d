# volt276d

Linux userspace daemon that reads HID reports from an SSL Volt 276 audio interface
and exposes its controls as standard kernel input events via uinput.

## What it does

The Volt 276 exposes compressor and vintage controls over USB HID, but the kernel
never maps them to input events. This daemon bridges the gap:

```
[SSL Volt 276]
    ↓  USB HID reports (/dev/hidraw*)
[volt276d]
    ↓  ioctls on /dev/uinput
[/dev/input/eventX]  ← visible to evtest, DAWs, any input consumer
```

## Controls mapped

| Control | Event type | Code |
|---|---|---|
| Compressor Input 1 (Off/Voc/GTR/Fast) | `EV_ABS` | `ABS_X` (0–3) |
| Compressor Input 2 (Off/Voc/GTR/Fast) | `EV_ABS` | `ABS_Y` (0–3) |
| Vintage Input 1 | `EV_SW` | SW slot 0 |
| Vintage Input 2 | `EV_SW` | SW slot 1 |

## Demo

```bash
# terminal 1
cargo run

# terminal 2 — find the device
grep -l "Volt 276 Controls" /sys/class/input/*/name | sed 's|.*/input\(.*\)/name|/dev/input/event\1|'
sudo evtest /dev/input/eventX

# touch the compressor or vintage buttons → events appear live
```

## Setup

```bash
# udev rules (run once, then replug the device)
sudo cp 99-volt276.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
sudo usermod -aG input $USER  # re-login required
```

## Architecture

```
src/
  main.rs       entry point, SIGINT handler
  uinput.rs     virtual input device (uinput ioctls, RAII cleanup)
  hid.rs        HID report reading loop
  protocol.rs   Volt 276 protocol constants (reverse engineered)
```

## Hardware

- **Device**: SSL Volt 276 — `VID:PID 2b5a:0023`
- **HID interface**: `/dev/hidraw*` — continuous stream of 14-byte reports
- **Gain knobs**: not exposed via HID (proprietary protocol on USB bulk endpoints)
