# control

Host-side Rust library + CLI (`promatrix`) for talking to a
Promatrix-4000-IO board over the USB CDC-ACM serial port its firmware
exposes: set outputs, request a status snapshot, and stream input-change
events. Speaks the wire protocol defined in `../protocol` — the same crate
the firmware itself uses, so the two can't drift apart.

## Prerequisites

Enter the Nix dev shell (`nix develop`, from the repo root) — it provides
the pinned Rust toolchain plus `pkg-config`/`udev`, which `serialport`'s
Linux backend needs to build.

## CLI

```sh
just run -- --port /dev/ttyACM0 set 3 on    # set Out3 high
just run -- --port /dev/ttyACM0 get         # print one STATUS snapshot
just run -- --port /dev/ttyACM0 watch       # snapshot, then stream EVT lines
```

(`just build`/`just build-release` build without running.) `--port` is
required; it can also be set once via the `PROMATRIX_PORT` env var instead
of passing `-p`/`--port` every time. The board's data interface is
typically `/dev/ttyACM0` and its log interface `/dev/ttyACM1` — see
`../firmware/README.md`.

`set` accepts `0`/`1`, `on`/`off`, or `true`/`false` for the value.

## Library

```rust
let mut client = control::Client::open("/dev/ttyACM0")?;
client.set_output(3, true)?;               // Out3 (1-indexed) on
let (outputs, inputs) = client.status()?;  // blocks for the STATUS reply
loop {
    match client.recv_event()? {
        control::Event::InputChanged { channel, value } => { /* channel is 0-indexed */ }
        control::Event::Status { outputs, inputs } => { /* bitmasks */ }
    }
}
```

`Client::open` starts a background thread that parses incoming lines and
delivers them as `Event`s through `recv_event`/`try_recv_event`; writes
(`set_output`/`get_status`/`status`) happen synchronously on the calling
thread. `status` sends `GetStatus` and blocks for the `Status` reply
specifically, discarding any unsolicited event (e.g. an `InputChanged`)
that happens to race ahead of it — use `get_status` + `recv_event`
directly if you want to see those too. Unrecognized lines from the device
(stray log output, anything the `protocol` crate's `parse_event` doesn't
understand) are silently dropped.

## Layout

* `src/lib.rs` – `Client`: opens the port, streams events off a background
  thread, sends commands.
* `src/main.rs` – `promatrix` CLI built on top of the library.
* `justfile` – build/run recipes.
