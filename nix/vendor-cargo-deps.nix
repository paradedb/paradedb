{
  lib,
  runCommand,
  rustPlatform,
}:
lockFile:
let
  lockFileContents = builtins.readFile lockFile;

  # importCargoLock uses name-version as its directory key. Give the DST
  # Proptest fork its own directory so it can coexist with crates.io Proptest.
  blocks = lib.splitString "\n[[package]]\n" lockFileContents;
  header = builtins.head blocks;
  packages = lib.partition (
    block:
    let
      package = builtins.fromTOML block;
    in
    package.name == "proptest"
    && lib.hasPrefix "git+https://github.com/antithesishq/proptest?" (package.source or "")
  ) (builtins.tail blocks);

  importLock = args: rustPlatform.importCargoLock ({ allowBuiltinFetchGit = true; } // args);
  selectPackages = blocks: lib.concatStringsSep "\n[[package]]\n" ([ header ] ++ blocks);
  workspaceVendor = importLock {
    lockFileContents = selectPackages packages.wrong;
  };
  dstVendor = importLock {
    lockFileContents = selectPackages packages.right;
  };
in
if packages.right == [ ] then
  importLock { inherit lockFile; }
else
  runCommand "cargo-vendor-dir" { } ''
    mkdir -p "$out/.cargo" "$out/dst"
    ln -s ${lockFile} "$out/Cargo.lock"
    for crate in ${workspaceVendor}/*/; do
      ln -s "''${crate%/}" "$out/"
    done

    sed 's|directory = "cargo-vendor-dir"|directory = "@vendor@"|' \
      ${workspaceVendor}/.cargo/config.toml > "$out/.cargo/config.toml"

    for crate in ${dstVendor}/*/; do
      ln -s "''${crate%/}" "$out/dst/"
      sed 's|replace-with = "vendored-sources"|replace-with = "vendored-dst"|' \
        "$crate/.cargo-config" >> "$out/.cargo/config.toml"
    done
    cat >> "$out/.cargo/config.toml" <<'CONFIG'

    [source.vendored-dst]
    directory = "@vendor@/dst"
    CONFIG
  ''
