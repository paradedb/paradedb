{
  description = "pg_search — Full text search for PostgreSQL using BM25";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # https://github.com/paradedb/paradedb/blob/dev/Cargo.lock — lindera-dictionary version
      linderaVersion = "1.4.1";
      linderaWebsite = "https://lindera.dev";

      # Lindera dictionaries — pre-fetched so the Nix sandbox build works
      # (lindera's build.rs downloads these at compile time)
      mkDictionaries =
        { fetchurl }:
        {
          # https://github.com/lindera/lindera/blob/v1.4.1/lindera-ko-dic/build.rs
          lindera-ko-dic = rec {
            language = "Korean";
            filename = "mecab-ko-dic-2.1.1-20180720.tar.gz";
            source = fetchurl {
              url = "${linderaWebsite}/${filename}";
              hash = "sha256-cCztIcYWfp2a68Z0q17lSvWNREOXXylA030FZ8AgWRo=";
            };
          };

          # https://github.com/lindera/lindera/blob/v1.4.1/lindera-cc-cedict/build.rs
          lindera-cc-cedict = rec {
            language = "Chinese";
            filename = "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz";
            source = fetchurl {
              url = "${linderaWebsite}/${filename}";
              hash = "sha256-7Tz54+yKgGR/DseD3Ana1DuMytLplPXqtv8TpB0JFsg=";
            };
          };

          # https://github.com/lindera/lindera/blob/v1.4.1/lindera-ipadic/build.rs
          lindera-ipadic = rec {
            language = "Japanese";
            filename = "mecab-ipadic-2.7.0-20250920.tar.gz";
            source = fetchurl {
              url = "${linderaWebsite}/${filename}";
              hash = "sha256-p7qfZF/+cJTlauHEqB0QDfj7seKLvheSYi6XKOFi2z0=";
            };
          };
        };

      mkPgSearch =
        {
          lib,
          buildPgrxExtension,
          cargo-pgrx_0_16_1,
          fetchurl,
          postgresql,
          stdenv,
          pkg-config,
          openssl,
          fontconfig,
        }:
        let
          dictionaries = mkDictionaries { inherit fetchurl; };
        in
        buildPgrxExtension {
          pname = "pg_search";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

          src = lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              let
                baseName = builtins.baseNameOf path;
              in
              # Exclude non-essential files from the source
              !(builtins.elem baseName [
                "flake.nix"
                "flake.lock"
                ".github"
                "docs"
                "node_modules"
                "target"
                ".git"
              ]);
          };

          # Use Cargo.lock directly — no vendored hash to maintain.
          # Only the git dependency outputHashes need updating when their revs change.
          # To get correct hashes, run: nix-prefetch-git <url> --rev <rev> | jq -r .hash
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "tantivy-0.26.0" = lib.fakeHash;
              "tantivy-fst-0.5.0" = lib.fakeHash;
            };
          };

          inherit postgresql;

          nativeBuildInputs = [ pkg-config ];
          buildInputs = [
            openssl
            fontconfig
          ];

          # Lindera dictionaries are copied to a temporary directory and the
          # LINDERA_CACHE environment variable prevents the build.rs files in
          # the Lindera crates from downloading their dictionary from an
          # external URL, which doesn't work in the Nix sandbox
          preConfigure = ''
            export LINDERA_CACHE=$TMPDIR/lindera-cache
            mkdir -p $LINDERA_CACHE/${linderaVersion}

            ${lib.concatMapStringsSep "\n" (dict: ''
              echo "Copying ${dict.language} dictionary to Lindera cache"
              cp ${dict.source} $LINDERA_CACHE/${linderaVersion}/${dict.filename}
            '') (lib.attrValues dictionaries)}

            echo "Lindera cache prepared at $LINDERA_CACHE"
          '';

          cargo-pgrx = cargo-pgrx_0_16_1;

          cargoPgrxFlags = [
            "--package"
            "pg_search"
          ];

          # Tests require a running PostgreSQL instance
          doCheck = false;

          meta = {
            description = "Full text search for PostgreSQL using BM25";
            homepage = "https://paradedb.com";
            license = lib.licenses.agpl3Only;
            platforms = supportedSystems;
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          pgVersions = {
            pg15 = pkgs.postgresql_15;
            pg16 = pkgs.postgresql_16;
            pg17 = pkgs.postgresql_17;
            pg18 = pkgs.postgresql_18;
          };
          mkForPg =
            pg:
            pkgs.callPackage mkPgSearch {
              postgresql = pg;
              inherit (pkgs) cargo-pgrx_0_16_1;
            };
        in
        {
          "pg_search-pg15" = mkForPg pgVersions.pg15;
          "pg_search-pg16" = mkForPg pgVersions.pg16;
          "pg_search-pg17" = mkForPg pgVersions.pg17;
          "pg_search-pg18" = mkForPg pgVersions.pg18;
          default = mkForPg pgVersions.pg18;
        }
      );
    };
}
