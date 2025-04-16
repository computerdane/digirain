{ rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "digirain";
  version = "main";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
}
