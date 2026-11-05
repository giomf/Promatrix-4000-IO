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
      # One package per host-side member of the root Cargo workspace.
      hostTool =
        name:
        pkgs.rustPlatform.buildRustPackage {
          pname = name;
          version = "0.1.0";
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./protocol
              ./promatrix-io
              ./promatrix-trigger
            ];
          };
          buildAndTestSubdir = name;
          cargoLock = {
            lockFile = ./Cargo.lock;
            # git deps (am03127-client) are fetched at the rev pinned in Cargo.lock.
            allowBuiltinFetchGit = true;
          };
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.udev ];
          meta.mainProgram = name;
        };
    in
    {
      packages.${system} = {
        promatrix-io = hostTool "promatrix-io";
        promatrix-trigger = hostTool "promatrix-trigger";
      };

      nixosModules = {
        promatrix-trigger = import ./nixos/trigger.nix self;
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
          # For the `promatrix-io` host tool: serialport's Linux backend
          # (libudev-sys, used for port enumeration) needs these to build.
          pkgs.pkg-config
          pkgs.udev
        ];
      };
    };
}
