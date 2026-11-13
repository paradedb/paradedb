{
  self,
  buildPgrxExtension,
  cargo-pgrx,
  fetchurl,
  lib,
  lld,
  llvmPackages,
  nix-update-script,
  pkg-config,
  openblas,
  stdenv,
  postgresql,
  writeShellScriptBin,
}:

let
  # Attribute set representing the Cargo.toml file in the root
  rootCargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);

  # Get the current Lindera version from the root Cargo.lock
  linderaVersion =
    (builtins.head (
      builtins.filter (p: p.name == "lindera")
        (builtins.fromTOML (builtins.readFile ../Cargo.lock)).package
    )).version;

  linderaWebsite = "https://lindera.dev";

  # pg_search's tokenizer uses several language dictionaries used by the Lindera crate
  dictionaries = {
    # https://github.com/lindera/lindera/blob/v6.2.0/lindera-ko-dic/build.rs#L15-L22
    lindera-ko-dic = rec {
      language = "Korean";
      filename = "mecab-ko-dic-2.1.1-20180720.tar.gz";
      source = fetchurl {
        url = "${linderaWebsite}/${filename}";
        hash = "sha256-cCztIcYWfp2a68Z0q17lSvWNREOXXylA030FZ8AgWRo=";
      };
    };

    # https://github.com/lindera/lindera/blob/v6.2.0/lindera-cc-cedict/build.rs#L15-L22
    lindera-cc-cedict = rec {
      language = "Chinese";
      filename = "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz";
      source = fetchurl {
        url = "${linderaWebsite}/${filename}";
        hash = "sha256-7Tz54+yKgGR/DseD3Ana1DuMytLplPXqtv8TpB0JFsg=";
      };
    };

    # https://github.com/lindera/lindera/blob/v6.2.0/lindera-ipadic/build.rs#L15-L22
    lindera-ipadic = rec {
      language = "Japanese";
      filename = "mecab-ipadic-2.7.0-20250920.tar.gz";
      source = fetchurl {
        url = "${linderaWebsite}/${filename}";
        hash = "sha256-p7qfZF/+cJTlauHEqB0QDfj7seKLvheSYi6XKOFi2z0=";
      };
    };
  };
in
assert cargo-pgrx.version == lib.removePrefix "=" rootCargoToml.workspace.dependencies.pgrx;
buildPgrxExtension (finalAttrs: {
  pname = "pg_search";
  version = rootCargoToml.workspace.package.version;
  src = self;

  # This hash needs to change any time the Rust dependencies are updated.
  # If maintainers forget to do so, Nix will throw an error message that begins
  # like this and then provides the correct new hash:
  # error: hash mismatch in fixed-output derivation '...'
  cargoHash = "sha256-TZ8hOalmbYugnFsj8GkoymkNASoaXFOgjc9T7TK7RY0=";

  inherit postgresql;
  # The Nix builder also passes cargoPgrxFlags to `stop`, which does not accept
  # packaging flags. Add those only to the package command.
  cargo-pgrx = writeShellScriptBin "cargo-pgrx" ''
    if [ "$1" = pgrx ] && [ "$2" = package ]; then
      set -- "$@" --no-assemble-unreleased --no-distrib-packages
    fi
    exec ${cargo-pgrx}/bin/cargo-pgrx "$@"
  '';

  # Lindera dictionaries are copied to a temporary directory and the
  # LINDERA_BUILD_DICTIONARY_CACHE_DIR environment variable prevents the
  # build.rs files in the Lindera crates from downloading their dictionary
  # from an external URL, which doesn't work in the Nix sandbox.
  preConfigure = ''
    export LINDERA_BUILD_DICTIONARY_CACHE_DIR=$TMPDIR/lindera-cache
    mkdir -p $LINDERA_BUILD_DICTIONARY_CACHE_DIR/${linderaVersion}

    ${lib.concatMapStringsSep "\n" (dict: ''
      echo "Copying ${dict.language} dictionary to Lindera cache"
      cp ${dict.source} $LINDERA_BUILD_DICTIONARY_CACHE_DIR/${linderaVersion}/${dict.filename}
    '') (lib.attrValues dictionaries)}

    echo "Lindera cache prepared at $LINDERA_BUILD_DICTIONARY_CACHE_DIR"
  '';

  cargoPgrxFlags = [
    "--package"
    "pg_search"
  ];

  buildInputs = lib.optionals (!stdenv.hostPlatform.isDarwin) [ openblas ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  CLANG_PATH = "${llvmPackages.clang}/bin/clang";

  # Keep the compiler, libclang and archiver on the same LLVM release.
  nativeBuildInputs = [
    lld
    llvmPackages.clang
    llvmPackages.llvm
    pkg-config
  ];

  # pgrx tests try to install the extension into postgresql nix store
  doCheck = false;

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Just use Postgres One Postgres for your application data, full-text search, vector retrieval, and aggregations.";
    homepage = "https://paradedb.com";
    changelog = "https://github.com/paradedb/paradedb/releases/tag/${finalAttrs.version}";
    license = lib.licenses.agpl3Only;
    # ../Cargo.toml#L14-L18
    broken = lib.versionOlder postgresql.version "15";
    platforms = postgresql.meta.platforms;
  };
})
