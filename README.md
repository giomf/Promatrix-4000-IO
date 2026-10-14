# Promatix-4000-IO

8-output / 7-input digital IO board (RP2040-based) with USB serial control.

* `PCB/` – KiCad hardware design.
* `firmware/` – Rust firmware (Embassy), see `firmware/README.md`.
* `protocol/` – wire protocol shared between firmware and `promatrix-io`.
* `promatrix-io/` – host-side Rust library + `promatrix-io` CLI, see `promatrix-io/README.md`.

Enter the Nix dev shell (`nix develop`) for the pinned toolchain and tools
used by all three Rust crates.
