{
  lib,
  rustPlatform,
  zsh,
}:

let
  cargoToml = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../shell
      ../tests
    ];
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeCheckInputs = [ zsh ];

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.package.repository;
    license = lib.licenses.mit;
    mainProgram = "zrs-zsh";
    platforms = lib.platforms.unix;
  };
}
