{
  description = "Lightweight text editor written with GTK4 + Rust";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = {flake-parts, ...} @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = {pkgs, ...}: let
        rustPlatform = pkgs.makeRustPlatform {
          inherit (pkgs) cargo rustc;
        };
      in {
        devShells = rec {
          default = stable;
          stable = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              rustc
              rustfmt
              rust-analyzer
            ];
          };
        };

        packages = rec {
          default = kyanite;
          kyanite = pkgs.callPackage ./kyanite.nix {
            inherit rustPlatform;
          };
        };
      };
    };
}
