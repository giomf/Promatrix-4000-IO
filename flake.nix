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
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "promatrix-control";
        version = "0.1.0";
        src = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset = pkgs.lib.fileset.unions [
            ./control
            ./protocol
          ];
        };
        cargoRoot = "control";
        buildAndTestSubdir = "control";
        cargoLock.lockFile = ./control/Cargo.lock;
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.udev ];
        meta.mainProgram = "promatrix";
      };

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
