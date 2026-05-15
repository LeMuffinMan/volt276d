# volt276d

A Linux daemon that exposes the hardware controls of a UA Volt 276 audio interface
as standard input events. Touch the compressor button or the vintage switch →
`evtest` (or any input-aware program) sees it immediately.

---

## The problem

The Volt 276 works on Linux. Recording, monitoring, adjusting levels — all fine.
But the hardware controls on the front panel — the compressor button, the vintage
switch — are invisible to the system:

```bash
$ amixer -c 1 controls
numid=3,iface=CARD,name='Universal Audio Internal Clock Validity'
numid=2,iface=PCM,name='Capture Channel Map'
numid=1,iface=PCM,name='Playback Channel Map'
```

Three controls, all of them metadata. No compressor. No vintage.

Linux sees the Volt as a generic USB audio device and handles the audio side.
But the Volt also sends control data over a separate USB HID interface — a
continuous byte stream encoding the state of every knob and button. That stream
is accessible in `/dev/hidraw*`, but nothing reads it. The protocol is
vendor-specific and the kernel has no mapping for it.

## What this daemon does

It reads the raw byte stream, decodes it, and creates a virtual input device
via **uinput** — a Linux kernel subsystem that lets userspace programs create
input devices. Anything that reads input events (evtest, a DAW, a script) sees
the virtual device exactly like a physical controller.

```
[UA Volt 276]
    ↓  USB HID byte stream  (/dev/volt276 → /dev/hidraw*)
[volt276d]
    ↓  ioctls on /dev/uinput
[/dev/input/eventX]  ← visible to evtest, DAWs, any input consumer
```

## Controls mapped

| Control | Event type | Code | Values |
|---|---|---|---|
| Compressor Input 1 | `EV_ABS` | `ABS_X` | 0=Off, 1=Voc, 2=GTR, 3=Fast |
| Compressor Input 2 | `EV_ABS` | `ABS_Y` | 0=Off, 1=Voc, 2=GTR, 3=Fast |
| Vintage Input 1 | `EV_SW` | SW slot 0 | 0=Off, 1=On |
| Vintage Input 2 | `EV_SW` | SW slot 1 | 0=Off, 1=On |

## How the protocol was found

The Volt streams 14-byte HID reports continuously. The first step was just
looking at what those bytes actually contain:

```bash
$ sudo cat /dev/hidraw7 | xxd | head -5
00000000: 0221 0202 0000 0000 0101 0000 0000      ..............
00000000: 0221 0202 0000 0000 0000 0000 0000      ..............
00000000: 0221 0202 0000 0000 0101 0000 0000      ..............
```

It streams constantly, even without touching anything, two alternating
patterns.

Pressing the compressor button while watching the stream: something changes,
but it scrolls too fast to see clearly. The next step was filtering out the
idle frames. The idle pattern starts with `02 21 02 02` — once identified,
filtering it isolates the events:

```bash
$ sudo cat /dev/hidraw7 | xxd | grep -v "0221 0202"
00000000: 0108 1400 0002 2111 0200 0000 0000      ......!.......
00000000: 0221 1102 0000 0000 0101 0000 0000      ..!...........
```

The second line is the compressor event. Comparing it to the idle frames,
byte[2] changed from `0x02` to `0x11`. Pressing Input 2's compressor instead:
byte[3] changes. Repeating this for the vintage switch: byte[1], two
independent bits (`0x40` for Input 1, `0x80` for Input 2).

Real report captured while pressing Compressor Input 1 (Voc) with Vintage Input 1 on.
Only buf[0..3] matter; the rest are ignored.

```
       [0]  [1]  [2]  [3]  [4]  [5]  [6]  [7]  [8]  [9]  [10] [11] [12] [13]
        02   61   11   02   00   00   00   00   01   01   00   00   00   00
        |    |    |    |
        |    |    |    └── buf[3]: compressor input 2 — 0x02 = Off
        |    |    └─────── buf[2]: compressor input 1 — 0x11 = Voc
        |    └─────────── buf[1]: vintage bits — 0x61 = 0x21|0x40 → input 1 ON
        └─────────────── buf[0]: report ID — only 0x02 processed
```

The full mapping is in [`src/protocol.rs`](src/protocol.rs).

## Setup

By default `/dev/hidraw*` requires root access. The setup below adds a udev
rule that grants group `input` access to the Volt's HID interface and creates
a stable symlink so the daemon doesn't depend on which `/dev/hidraw*` number
the kernel assigns at boot.

Create `/etc/udev/rules.d/99-volt276.rules`:

```
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="2b5a", ATTRS{idProduct}=="0023", \
  GROUP="input", MODE="0660", SYMLINK+="volt276"
```

Then apply:

```bash
sudo udevadm control --reload && sudo udevadm trigger
sudo usermod -aG input $USER  # log out and back in
```

After replug, `/dev/volt276` points to the correct hidraw device.

## Demo

```bash
# terminal 1
cargo run

# terminal 2 — find the virtual device
grep -l "Volt 276 Controls" /sys/class/input/*/name \
  | sed 's|.*/input\(.*\)/name|/dev/input/event\1|'
sudo evtest /dev/input/eventX

# touch the compressor or vintage buttons → events appear live
```

## Architecture

```
src/
  main.rs       entry point, signal handler
  uinput.rs     virtual input device (uinput ioctls)
  hid.rs        HID report reading loop
  protocol.rs   byte mapping (reverse engineered)
```

## Hardware

- **Device**: UA Volt 276 — `VID:PID 2b5a:0023`
- **HID interface**: `/dev/hidraw*` (symlinked to `/dev/volt276` after setup)
- **Gain knobs**: not mapped — sent on a separate USB endpoint, protocol TBD
