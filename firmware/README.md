# Promatrix-4000-IO Firmware

Rust firmware for the RP2040 Zero (Waveshare), built on [Embassy](https://embassy.dev).

It controls 8 digital outputs (via a ULN2803A Darlington array) and senses 7
digital inputs (via PC817 optocouplers) in near real-time, and exposes both
to a host PC over a USB CDC-ACM ("virtual serial port") connection. A second
CDC-ACM interface carries plain-text logs.

## Prerequisites

Enter the Nix dev shell (`nix develop`) which provides the pinned Rust
toolchain (with the `thumbv6m-none-eabi` target), `picotool` and `just` for
flashing.

Flashing goes through the RP2040's built-in USB bootloader rather than a
debug probe. A board already running this firmware reboots itself into
BOOTSEL mode on request (see `src/reset.rs`), so `just flash`/`flash-release`
work as-is with no button-pressing. For a blank board, or one stuck in a
panic, hold the BOOT button while plugging in (or resetting) it so it
enumerates as `RP2 Boot`, then flash. If `picotool` reports a permissions
error, install its udev rule (`services.udev.packages = [ pkgs.picotool ];`
on NixOS) so your user can access the device without `sudo`.

## Build & flash

```sh
just flash          # debug build
just flash-release  # release build
```

(`just build`/`just build-release` build without flashing.) This runs
`picotool load -f -x -t elf ...` to flash the ELF and boot into it; see
`justfile`.

## Logs

The board must be in a normal, running state (not BOOTSEL) for logs to
appear. Logs are plain ASCII text on a second CDC-ACM serial port — no
special decoding needed:

```sh
just logs                  # defaults to /dev/ttyACM1
just logs /dev/ttyACM0     # override if the port numbering differs
```

See `src/log.rs` for the logging implementation and `src/usb.rs` for how the
two CDC-ACM interfaces are set up.

## Pin mapping

| Logical channel | GPIO   | Notes                                    |
|------------------|--------|-------------------------------------------|
| Out1..Out8       | 0..7   | direct, logical "on" == GPIO high         |
| In1              | 10     | electrical low == input active            |
| In2              | 9      | electrical low == input active            |
| In3              | 8      | electrical low == input active            |
| In4              | 14     | electrical low == input active            |
| In5              | 13     | electrical low == input active            |
| In6              | 12     | electrical low == input active            |
| In7              | 11     | electrical low == input active            |
| In8              | —      | not wired on this board revision          |

See `src/io.rs` for details on the output/input electrical polarity.

## USB protocol

The device enumerates as a USB CDC-ACM serial port (VID/PID
`16c0:27dd`, the pid.codes "Test PID" — replace with a real allocation before
shipping). Talk to it with any serial terminal/library using a simple,
line-based (`\n`-terminated) text protocol:

Host -> device:

```
SET <channel> <0|1>   set output <channel> (1..=8) to 0 or 1
GET                    request a full status snapshot
```

Device -> host:

```
EVT IN <channel> <0|1>            input <channel> (1..=7) changed, sent as soon as detected (debounced ~5ms)
STATUS OUT <8 bits> IN <7 bits>   full snapshot, sent in response to GET, MSB = highest channel
```

See `src/protocol.rs` for the exact grammar.

## Layout

* `src/main.rs` – entry point, peripheral init and task spawning.
* `src/io.rs` – physical GPIO <-> logical channel mapping for outputs/inputs.
* `src/usb.rs` – USB CDC-ACM device/class setup (data + log interfaces).
* `src/log.rs` – plain-text logging over the second CDC-ACM interface.
* `src/protocol.rs` – host <-> device wire protocol (parsing/formatting).
* `src/reset.rs` – vendor USB interface letting `picotool -f` reboot the
  board into BOOTSEL mode without the BOOT button.
* `src/channels.rs` – inter-task queues (parsed commands, outgoing events).
* `src/state.rs` – shared atomic snapshot of current output/input state.
* `src/tasks.rs` – async tasks: USB reader/writer, output command handler,
  one input-watcher task per input pin.
* `memory.x` / `build.rs` – RP2040 linker memory layout.
* `.cargo/config.toml` – build target and `cargo run`'s `picotool` runner.
* `justfile` – build/flash/log recipes (see `just --list`).
