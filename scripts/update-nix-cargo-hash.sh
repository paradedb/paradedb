#!/usr/bin/env bash
#
# Updates the cargoHash in nix/pg_search.nix by building the cargoDeps
# derivation with a fake hash and extracting the correct one from the error.
#
# Usage: ./scripts/update-nix-cargo-hash.sh [pg_version]
#   pg_version: PostgreSQL major version (default: 18)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NIX_FILE="$REPO_ROOT/nix/pg_search.nix"
PG_VERSION="${1:-18}"
FAKE_HASH="sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="

if ! command -v nix &>/dev/null; then
  echo "Error: nix is not installed" >&2
  exit 1
fi

# Read current hash (portable: no grep -P)
current_hash=$(sed -n 's/.*cargoHash = "\([^"]*\)".*/\1/p' "$NIX_FILE")
echo "Current cargoHash: $current_hash"

# Temporarily set a fake hash to force nix to report the correct one
cp "$NIX_FILE" "$NIX_FILE.bak"
sed "s|cargoHash = \"$current_hash\"|cargoHash = \"$FAKE_HASH\"|" "$NIX_FILE.bak" >"$NIX_FILE"

# Build cargoDeps and capture the correct hash from the error
echo "Computing correct cargoHash (this may take a while)..."
correct_hash=""
if build_output=$(cd "$REPO_ROOT" && nix build --no-link ".#pg_search-pg${PG_VERSION}.cargoDeps" 2>&1); then
  # Build succeeded with fake hash — shouldn't happen, restore and exit
  echo "Warning: build succeeded with fake hash, restoring original" >&2
  mv "$NIX_FILE.bak" "$NIX_FILE"
  exit 1
else
  correct_hash=$(echo "$build_output" | sed -n 's/.*got: *//p' | head -1 | tr -d '[:space:]')
fi

# Restore the backup
mv "$NIX_FILE.bak" "$NIX_FILE"

if [[ -z "$correct_hash" ]]; then
  echo "Error: could not extract correct hash from nix output" >&2
  echo "Build output:" >&2
  echo "$build_output" >&2
  exit 1
fi

if [[ "$current_hash" == "$correct_hash" ]]; then
  echo "cargoHash is already up to date."
  exit 0
fi

# Update the file with the correct hash
sed "s|cargoHash = \"$current_hash\"|cargoHash = \"$correct_hash\"|" "$NIX_FILE" >"$NIX_FILE.tmp"
mv "$NIX_FILE.tmp" "$NIX_FILE"
echo "Updated cargoHash: $current_hash -> $correct_hash"
