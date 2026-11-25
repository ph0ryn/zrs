{
  lib,
  rustPlatform,
  makeWrapper,
  fzf,
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

  nativeBuildInputs = [
    makeWrapper
  ];

  postInstall = ''
    wrapProgram "$out/bin/zrs" \
      --suffix PATH : "${lib.makeBinPath [ fzf ]}"
  '';

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.package.repository;
    mainProgram = "zrs";
    platforms = lib.platforms.unix;
  };
}
