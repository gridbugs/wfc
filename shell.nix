# Nix shell with rust compiler and dependencies for libraries and examples

let
  # This overlay is mozilla/nixpkgs-mozilla with the patch applied from this PR:
  # https://github.com/mozilla/nixpkgs-mozilla/pull/250
  # ...which replaces deprecated usage of stdenv.lib with lib.
  moz_overlay_url = "https://github.com/gridbugs/nixpkgs-mozilla/archive/with-stdenv.lib-fix.tar.gz";
  moz_overlay = import (builtins.fetchTarball moz_overlay_url);
  nixpkgs = import <nixpkgs> {
    overlays = [ moz_overlay ];
  };
  ruststable = (nixpkgs.latest.rustChannels.stable.rust.override {
    extensions = [ "rust-src" "rust-analysis" ];
  });
in
with nixpkgs;
stdenv.mkDerivation rec {
  name = "moz_overlay_shell";
  buildInputs = [
    ruststable
  ];

  # Enable backtraces on panics
  RUST_BACKTRACE = 1;

  # Without this graphical frontends can't find the GPU adapters
  LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
}
