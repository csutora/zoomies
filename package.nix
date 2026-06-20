{ lib, rustPlatform, pkg-config, wayland }:

rustPlatform.buildRustPackage {
  pname = "zoomies";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./src ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ wayland ];

  meta = {
    description = "momentum scrolling for touchpads on wlroots-based wayland compositors";
    license = lib.licenses.mit;
    mainProgram = "zoomies";
    platforms = lib.platforms.linux;
  };
}
