#!/usr/bin/env bash
set -euo pipefail

operation="$1"
repository="$2"
auth=(-c http.extraheader=)
if [[ -n "${APP_TOKEN:-}" ]]; then
  auth=(-c "http.extraheader=Authorization: Basic $(printf 'x-access-token:%s' "$APP_TOKEN" | base64 | tr -d '\n')")
fi

case "$operation" in
  checkout)
    case "$3" in
      benchmarker) directory=benchmarker ;;
      queries) directory=benchmarks ;;
      stressgres) directory=stressgres ;;
      *)
        echo "Unknown benchmark type: $3" >&2
        exit 1
        ;;
    esac
    git "${auth[@]}" clone --filter=blob:none --no-checkout --depth=1 --single-branch --branch gh-pages \
      "$repository" benchmark-data-repository
    git -C benchmark-data-repository "${auth[@]}" sparse-checkout set --no-cone "/${directory}/"
    git -C benchmark-data-repository "${auth[@]}" checkout
    ;;
  dashboards)
    destination="benchmarker-runs/$2/$3"
    mkdir -p "benchmark-data-repository/${destination}"
    cp _ghpages_html/*.html "benchmark-data-repository/${destination}/"
    git -C benchmark-data-repository add --sparse -- "$destination"
    ;;
  push)
    cd benchmark-data-repository
    if git diff --cached --quiet; then
      echo "No benchmark changes to publish"
      exit 0
    fi
    git -c user.name='github-actions[bot]' -c user.email='github-actions[bot]@users.noreply.github.com' \
      commit -m "Publish benchmark results $3"
    git "${auth[@]}" push "$repository" HEAD:gh-pages
    ;;
  *)
    echo "Unknown publication operation: $operation" >&2
    exit 1
    ;;
esac
