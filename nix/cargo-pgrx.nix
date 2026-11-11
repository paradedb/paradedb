{
  lib,
  fetchurl,
  runCommand,
  rustPlatform,
  pkg-config,
  zstd,
}:
let
  version = "0.20.0-beta.1";
  archive = fetchurl {
    url = "https://static.crates.io/crates/cargo-pgrx/cargo-pgrx-${version}.crate";
    hash = "sha256-2fS+fHgNd5K6rX5gMPvSFllcDA17iph63t7AvCptkaM=";
  };
  # Use the published crate's lockfile without maintaining another copy here.
  source = runCommand "cargo-pgrx-${version}-source" { } ''
    mkdir -p $out
    tar -xzf ${archive} --strip-components=1 -C $out
  '';
in
rustPlatform.buildRustPackage {
  pname = "cargo-pgrx";
  inherit version;
  src = source;
  cargoLock.lockFile = "${source}/Cargo.lock";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ zstd ];
  cargoBuildFlags = [ "--bin=cargo-pgrx" ];
  # CLI integration tests require PostgreSQL installations outside this derivation.
  doCheck = false;
  meta = {
    description = "Build and manage pgrx PostgreSQL extensions";
    mainProgram = "cargo-pgrx";
    license = lib.licenses.mit;
  };
}
