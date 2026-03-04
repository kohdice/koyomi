{
  description = "Koyomi";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable."1.92.0".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "rustfmt"
            "clippy"
          ];
        };
        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs.nixfmt.enable = true;
          programs.rustfmt = {
            enable = true;
            package = rust;
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "koyomi";

          nativeBuildInputs = with pkgs; [
            rust
            cargo-deny
          ];
        };

        formatter = treefmtEval.config.build.wrapper;
      }
    );
}
