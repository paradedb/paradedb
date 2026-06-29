#!/usr/bin/env bash
#
# Run docker/generate-dockerfiles.sh to generate every flavor of Dockerfile we support from docker/Dockerfile.template.
# Docker Official Images does not support build arguments in Dockerfiles, so we have to generate a separate file for each
# Postgres version we support. Our normal Dockerfile includes Barman cloud which is specific to our deployment approach
# and thus cannot be included in the official images. We also have a version of the file for use in Antithesis.
#
# This script downloads the .debs for the specified version from GitHub Releases in order to compute checksums and embed
# them in the Dockerfiles.
#
# To make a change, update the template, rerun the script with the desired version, and commit the generated Dockerfiles.

set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Error: PG_SEARCH_VERSION is required"
  echo "Usage: $0 <pg_search_version> [pg_majors]"
  echo "Example: $0 0.24.0              # generate all supported PG majors"
  echo "Example: $0 0.24.0-rc.1 \"18\"  # generate only PG 18 (e.g. beta releases)"
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
all_versions=(15 16 17 18)
# `latest_version` decides which PG major gets the Antithesis flavor; keep it tied to the true latest
# regardless of any version restriction below.
latest_version=${all_versions[${#all_versions[@]} - 1]}
PG_SEARCH_VERSION="${1}"

# Optionally restrict which PG majors to generate (space-separated), e.g. "18" for beta releases that
# only ship a PG 18 Docker image. Defaults to all supported majors.
if [[ -n "${2:-}" ]]; then
  read -r -a versions <<< "${2}"
else
  versions=("${all_versions[@]}")
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

fetch_checksum() {
  local pg_version="$1"
  local arch="$2"
  local release_asset_name="postgresql-${pg_version}-pg-search_${PG_SEARCH_VERSION}-1PARADEDB-trixie_${arch}.deb"
  local deb="${tmp_dir}/${release_asset_name}"

  echo "Fetching ${release_asset_name}" >&2
  curl -fsSL -o "$deb" "https://github.com/paradedb/paradedb/releases/download/v${PG_SEARCH_VERSION}/${release_asset_name}" || return 1
  sha256sum "$deb" | awk '{ print $1 }'
}

render() {
  local flavor="$1"
  local pg_version="$2"
  local pg_search_deb_amd64_sha256="$3"
  local pg_search_deb_arm64_sha256="$4"
  local output="${script_dir}/Dockerfile.$flavor-$pg_version"

  awk \
    -v pg_version="$pg_version" \
    -v pg_search_version="$PG_SEARCH_VERSION" \
    -v pg_search_deb_amd64_sha256="$pg_search_deb_amd64_sha256" \
    -v pg_search_deb_arm64_sha256="$pg_search_deb_arm64_sha256" \
  -v flavor="$flavor" '
      BEGIN { include = 1 }
      /^# %%ANTITHESIS_BEGIN%%$/ { include = flavor == "antithesis"; next }
      /^# %%BARMAN_BEGIN%%$/ { include = flavor != "official"; next }
      /^# %%STANDARD_BEGIN%%$/ { include = flavor != "antithesis"; next }
      /^# %%(ANTITHESIS|BARMAN|STANDARD)_END%%$/ { include = 1; next }
      !include { next }
      {
        gsub(/@@PG_VERSION_MAJOR@@/, pg_version)
        gsub(/@@PG_SEARCH_VERSION@@/, pg_search_version)
        gsub(/@@PG_SEARCH_DEB_AMD64_SHA256@@/, pg_search_deb_amd64_sha256)
        gsub(/@@PG_SEARCH_DEB_ARM64_SHA256@@/, pg_search_deb_arm64_sha256)
        print
      }
    ' "${script_dir}/Dockerfile.template" > "$output"
}

for pg_version in "${versions[@]}"; do
  pg_search_deb_amd64_sha256="$(fetch_checksum "$pg_version" amd64)"
  pg_search_deb_arm64_sha256="$(fetch_checksum "$pg_version" arm64)"

  render paradedb "$pg_version" "$pg_search_deb_amd64_sha256" "$pg_search_deb_arm64_sha256"
  render official "$pg_version" "$pg_search_deb_amd64_sha256" "$pg_search_deb_arm64_sha256"
  if [[ $pg_version -eq $latest_version ]]; then
    render antithesis "$pg_version" "$pg_search_deb_amd64_sha256" "$pg_search_deb_arm64_sha256"
  fi
done
