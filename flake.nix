{
  description = "pg_search: full-text search for PostgreSQL using BM25";

  # Flake inputs
  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1"; # Unstable Nixpkgs

    # Fenix: a toolkit for building Rust toolchains for Nix
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # Flake outputs
  outputs =
    { self, ... }@inputs:
    let
      # The systems supported for this flake's outputs
      supportedSystems = [
        "x86_64-linux" # 64-bit Intel/AMD Linux
        "aarch64-linux" # 64-bit ARM Linux
        "aarch64-darwin" # 64-bit ARM macOS
      ];

      # A helper for providing system-specific attributes
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            # Provides a system-specific, configured Nixpkgs
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            };
          }
        );
    in
    {
      # This package output enables you to build pg_search locally:
      # NIXPKGS_ALLOW_BROKEN=1 nix build --impure .#pg_search
      # Or you can build the `default` package like this for the sake of brevity:
      # NIXPKGS_ALLOW_BROKEN=1 nix build
      # The "allow broken" setting is necessary because *all* PostgreSQL plugins based on Nixpkgs
      # are technically broken because tests require a running instance of PostgreSQL in the
      # Nix sandbox, which is generally infeasible.
      packages = forEachSupportedSystem (
        { pkgs, system }:
        {
          default = self.packages.${system}.pg_search;
          pg_search = pkgs.callPackage ./nix/pg_search.nix { };
        }
      );

      # Development environments output by this flake

      # To activate the default environment:
      # nix develop
      # Or if you use direnv:
      # direnv allow
      devShells = forEachSupportedSystem (
        { pkgs, system }:
        {
          # Run `nix develop` to activate this environment or `direnv allow` if you have direnv installed
          default = pkgs.mkShellNoCC {
            # The Nix packages provided in the environment
            packages = with pkgs; [
              # Fenix-based Rust toolchain
              rustToolchain

              # Add the official Nix formatter to the environment
              self.formatter.${system}

              # Adds PostgreSQL related tools to the environment (pg_ctl, psql, etc)
              postgresql

              # Makefile tools
              postgresql.pg_config
              perl
              cargo-pgrx_0_16_1
            ];

            # Environment variables the environment
            env.PGRX_HOME = ".pgrx";

            # Shell logic executed when the environment is activated
            shellHook = "";
          };
        }
      );

      # Nix formatter

      # This applies the formatter that follows RFC 166, which defines a standard format for Nix:
      # https://github.com/NixOS/rfcs/pull/166

      # To format all Nix files:
      # git ls-files -z '*.nix' | xargs -0 -r nix fmt
      # To check formatting:
      # git ls-files -z '*.nix' | xargs -0 -r nix develop --command nixfmt --check
      formatter = forEachSupportedSystem ({ pkgs, ... }: pkgs.nixfmt);

      # A Nixpkgs overlay that adds a Fenix-based Rust toolchain
      overlays.default = final: prev: {
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            # Use stable Rust plus pinned versions of the rest of the toolchain
            with stable;
            [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
              rust-analyzer
            ]
          );
      };
    };
}
