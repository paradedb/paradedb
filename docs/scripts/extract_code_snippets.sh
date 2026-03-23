#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SOURCE_ROOT="${1:-$REPO_ROOT}"
OUTPUT_DIR="${2:-$REPO_ROOT/extracted-code-snippets}"

mkdir -p "$OUTPUT_DIR"

SQL_OUT="$OUTPUT_DIR/sql.txt"
DJANGO_OUT="$OUTPUT_DIR/django.txt"
RAILS_OUT="$OUTPUT_DIR/rails.txt"

: >"$SQL_OUT"
: >"$DJANGO_OUT"
: >"$RAILS_OUT"

doc_count=0

while IFS= read -r rel_path; do
  doc_count=$((doc_count + 1))
  rel_path="${rel_path#./}"

  awk \
    -v rel_path="$rel_path" \
    -v sql_out="$SQL_OUT" \
    -v django_out="$DJANGO_OUT" \
    -v rails_out="$RAILS_OUT" '
    function classify(info, lower, parts, lang) {
      lower = tolower(info)
      split(lower, parts, /[[:space:]]+/)
      lang = parts[1]

      if (lang == "sql") {
        return "sql"
      }
      if (lang == "python" && lower ~ /(^|[[:space:]])django([[:space:]]|$)/) {
        return "django"
      }
      if (lang == "ruby" && lower ~ /(^|[[:space:]])rails([[:space:]]|$)/) {
        return "rails"
      }

      return ""
    }

    function reset_group(    i) {
      delete group_targets
      delete group_bodies
      delete seen_targets
      group_snippet_count = 0
      group_target_count = 0
      for (i = 1; i <= snippet_len; i++) {
        delete snippet_lines[i]
      }
      snippet_len = 0
      target = ""
    }

    function finish_fence(    body) {
      if (target == "" || snippet_len == 0 || !in_codegroup) {
        target = ""
        snippet_len = 0
        delete snippet_lines
        return
      }

      body = ""
      for (i = 1; i <= snippet_len; i++) {
        body = body snippet_lines[i] "\n"
      }

      group_targets[++group_snippet_count] = target
      group_bodies[group_snippet_count] = body

      if (!(target in seen_targets)) {
        seen_targets[target] = 1
        group_target_count++
      }

      target = ""
      snippet_len = 0
      delete snippet_lines
    }

    function flush_group(    idx, output_file, body) {
      if (group_target_count < 2) {
        reset_group()
        return
      }

      for (idx = 1; idx <= group_snippet_count; idx++) {
        if (group_targets[idx] == "sql") {
          output_file = sql_out
        } else if (group_targets[idx] == "django") {
          output_file = django_out
        } else if (group_targets[idx] == "rails") {
          output_file = rails_out
        } else {
          continue
        }

        printf "===== %s (group %d) =====\n", rel_path, codegroup_index >> output_file
        printf "%s\n", group_bodies[idx] >> output_file
      }

      reset_group()
    }

    /<CodeGroup[ >]/ {
      if (in_codegroup) {
        flush_group()
      }
      in_codegroup = 1
      codegroup_index++
      reset_group()
      next
    }

    /<\/CodeGroup>/ {
      if (in_fence) {
        finish_fence()
        in_fence = 0
      }
      if (in_codegroup) {
        flush_group()
        in_codegroup = 0
      }
      next
    }

    /^```/ {
      if (!in_codegroup) {
        next
      }

      if (in_fence) {
        finish_fence()
        in_fence = 0
        next
      }

      in_fence = 1
      info = substr($0, 4)
      sub(/^[[:space:]]+/, "", info)
      target = classify(info)
      next
    }

    {
      if (in_codegroup && in_fence && target != "") {
        snippet_lines[++snippet_len] = $0
      }
    }

    END {
      if (in_fence) {
        finish_fence()
      }
      if (in_codegroup) {
        flush_group()
      }
    }
  ' "$SOURCE_ROOT/$rel_path"
done < <(cd "$SOURCE_ROOT" && find . -path './legacy' -prune -o -type f -name '*.mdx' -print | LC_ALL=C sort)

if [[ $doc_count -eq 0 ]]; then
  echo "No .mdx files found under $SOURCE_ROOT" >&2
  exit 1
fi

sql_count="$(grep -c '^===== ' "$SQL_OUT" || true)"
django_count="$(grep -c '^===== ' "$DJANGO_OUT" || true)"
rails_count="$(grep -c '^===== ' "$RAILS_OUT" || true)"

echo "Wrote $sql_count SQL snippets to $SQL_OUT"
echo "Wrote $django_count Django snippets to $DJANGO_OUT"
echo "Wrote $rails_count Rails snippets to $RAILS_OUT"
