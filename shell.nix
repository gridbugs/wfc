# Nix shell with rust compiler and dependencies for libraries and examples

let
  moz_overlay_url = "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz";
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

    # Needed for graphical examples
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    vulkan-loader
    vulkan-tools
    libGL
  ];

  # Enable backtraces on panics
  RUST_BACKTRACE = 1;

  # Without this graphical frontends can't find the GPU adapters
  LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
}
