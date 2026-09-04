{
  description = "A simple rust flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./firmware/rust-toolchain.toml;
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          toolchain
          pkgs.cargo-sort
          pkgs.cargo-edit
          pkgs.probe-rs-tools
          pkgs.picotool
          pkgs.just
          pkgs.picocom
          # For the `control` host tool: serialport's Linux backend
          # (libudev-sys, used for port enumeration) needs these to build.
          pkgs.pkg-config
          pkgs.udev
        ];
      };
    };
}
