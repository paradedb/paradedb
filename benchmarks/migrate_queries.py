#!/usr/bin/env python3
"""Migrates benchmark query files from flat multi-query .sql files into nested directories.

Directory layout:
  queries/
    <query_name>/
      README.md
      <variant_1>.sql
      <variant_2>.sql
      ...
"""

import argparse
import glob
import os
import sys

# Lookup table for multi-query files mapping (filename, statement_offset) -> variant_name.
#
# Single-query files are not looked up here and remain flat .sql files.
# All multi-query variants are preserved with their respective names (e.g. postgres, hash_partitioned).
VARIANT_LOOKUP: dict[tuple[str, int], str | None] = {
    # -------------------------------------------------------------------------
    # Join queries (16 files)
    # -------------------------------------------------------------------------
    # Disjunctive local sort: local fast-field sort isolates pure join and filter evaluation.
    (
        "join_disjunctive_local_sort.sql",
        0,
    ): "postgres",  # Unhinted Postgres join plan (custom scan off)
    (
        "join_disjunctive_local_sort.sql",
        1,
    ): "hash_partitioned",  # ParadeDB default hash-partitioned join scan
    (
        "join_disjunctive_local_sort.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan optimization
    # Foreign filter, local sort: filter parent table (users), sort child table (posts).
    (
        "join_foreign_filter_local_sort.sql",
        0,
    ): "postgres",  # Postgres nested loop / hash join baseline
    (
        "join_foreign_filter_local_sort.sql",
        1,
    ): "hash_partitioned",  # Hash-partitioned join scan
    (
        "join_foreign_filter_local_sort.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan
    # Semi-join filter: list-logic subquery filter with title sort.
    ("join_semi_filter.sql", 0): "postgres",  # Postgres semi-join baseline
    (
        "join_semi_filter.sql",
        1,
    ): "hash_partitioned",  # Join scan with sortedness disabled
    ("join_semi_filter.sql", 2): "range_partitioned",  # Range-partitioned join scan
    (
        "join_semi_filter.sql",
        3,
    ): "term_set",  # pdb.term_set workaround pushing semi-join to search index
    # Scalar COUNT(*) on join.
    (
        "join_aggregate_count.sql",
        0,
    ): "postgres",  # Postgres standard aggregate over join
    (
        "join_aggregate_count.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_count.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Date histogram on join.
    (
        "join_aggregate_date_histogram.sql",
        0,
    ): "postgres",  # Postgres standard aggregate over join
    (
        "join_aggregate_date_histogram.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_date_histogram.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Disjunctive search scalar COUNT(*) aggregate on join.
    (
        "join_aggregate_disjunctive_count.sql",
        0,
    ): "postgres",  # Postgres standard aggregate over join
    (
        "join_aggregate_disjunctive_count.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_disjunctive_count.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # GROUP BY aggregate on join.
    ("join_aggregate_groupby.sql", 0): "postgres",  # Postgres hash aggregate over join
    (
        "join_aggregate_groupby.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_groupby.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Multiple aggregates on join (SUM, COUNT, MIN, MAX).
    ("join_aggregate_multi.sql", 0): "postgres",  # Postgres aggregate over join
    (
        "join_aggregate_multi.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_multi.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Aggregate sort (computed property): ordering by an aggregated value on join.
    ("join_aggregate_sort.sql", 0): "postgres",  # Postgres standard aggregate + sort
    (
        "join_aggregate_sort.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan over join
    (
        "join_aggregate_sort.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Top-K aggregate on join: GROUP BY high-cardinality badges with COUNT(*) and LIMIT 10.
    ("join_aggregate_topk_count.sql", 0): "postgres",  # Postgres aggregate over join
    (
        "join_aggregate_topk_count.sql",
        1,
    ): "aggregate_scan",  # ParadeDB Top-K aggregate scan
    (
        "join_aggregate_topk_count.sql",
        2,
    ): "aggregate_scan_late_materialized",  # Aggregate scan with strings kept late-materialized
    # Multi-facet window aggregates on join (COUNT over PARTITION BY).
    (
        "join_aggregate_window_facet.sql",
        0,
    ): "postgres",  # Postgres window aggregate baseline
    (
        "join_aggregate_window_facet.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan enabled
    (
        "join_aggregate_window_facet.sql",
        2,
    ): "range_partitioned",  # Aggregate scan with range-partitioned join
    # Conjunctive search with score sort: sum of scores across join tables.
    # Preserved as postgres.sql with comments explaining why it cannot be planned without joinscan.
    (
        "join_conjunctive_score_sort.sql",
        0,
    ): "postgres",  # Preserved Postgres placeholder query with explanatory note
    (
        "join_conjunctive_score_sort.sql",
        1,
    ): "hash_partitioned",  # Hash-partitioned join scan with score sorting
    (
        "join_conjunctive_score_sort.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan with score sorting
    # Disjunctive search with score sort: sum of scores across disjunctive join tables.
    # Preserved as postgres.sql with comments explaining why it cannot be planned without joinscan.
    (
        "join_disjunctive_score_sort.sql",
        0,
    ): "postgres",  # Preserved Postgres placeholder query with explanatory note
    (
        "join_disjunctive_score_sort.sql",
        1,
    ): "hash_partitioned",  # Hash-partitioned join scan with score sorting
    (
        "join_disjunctive_score_sort.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan with score sorting
    # Distinct parent sort: join explosion deduplicated by DISTINCT parent property.
    (
        "join_distinct_parent_sort.sql",
        0,
    ): "postgres",  # Postgres distinct aggregate baseline
    (
        "join_distinct_parent_sort.sql",
        1,
    ): "hash_partitioned",  # Hash-partitioned join scan
    (
        "join_distinct_parent_sort.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan
    # Permissioned search: filtering on parent permission while scoring child documents.
    ("join_permissioned_search.sql", 0): "postgres",  # Postgres join baseline
    (
        "join_permissioned_search.sql",
        1,
    ): "hash_partitioned",  # Hash-partitioned join scan
    (
        "join_permissioned_search.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan
    # Top-K by score restricted by a join: score-driven sort on parent table.
    (
        "join_top_k-score-desc-high-selectivity.sql",
        0,
    ): "postgres",  # Postgres-driven join (scan off)
    (
        "join_top_k-score-desc-high-selectivity.sql",
        1,
    ): "hash_partitioned",  # pg-search join scan on
    (
        "join_top_k-score-desc-high-selectivity.sql",
        2,
    ): "range_partitioned",  # Range-partitioned join scan on
    # -------------------------------------------------------------------------
    # Top-K BM25 score queries with parallel vs single-worker variants (9 files)
    # -------------------------------------------------------------------------
    ("top_k-score-asc.sql", 0): "default",  # Parallel Gather (default workers)
    ("top_k-score-asc.sql", 1): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-asc-high-selectivity.sql",
        0,
    ): "default",  # Parallel Gather (default workers)
    (
        "top_k-score-asc-high-selectivity.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-asc-medium-selectivity.sql",
        0,
    ): "default",  # Parallel Gather (default workers)
    (
        "top_k-score-asc-medium-selectivity.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    ("top_k-score-desc.sql", 0): "default",  # Parallel Gather (default workers)
    ("top_k-score-desc.sql", 1): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-desc-high-selectivity.sql",
        0,
    ): "default",  # Parallel Gather (default workers)
    (
        "top_k-score-desc-high-selectivity.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-desc-medium-selectivity.sql",
        0,
    ): "default",  # Parallel Gather (default workers)
    (
        "top_k-score-desc-medium-selectivity.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-desc-tiebreaker.sql",
        0,
    ): "default",  # Parallel Gather (BMW with tiebreaker)
    (
        "top_k-score-desc-tiebreaker.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-multi-term-asc.sql",
        0,
    ): "default",  # Parallel Gather (multi-term BM25)
    (
        "top_k-score-multi-term-asc.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    (
        "top_k-score-multi-term-desc.sql",
        0,
    ): "default",  # Parallel Gather (multi-term BM25)
    (
        "top_k-score-multi-term-desc.sql",
        1,
    ): "single_worker",  # max_parallel_workers_per_gather=0
    # -------------------------------------------------------------------------
    # Sum aggregates (3 files)
    # -------------------------------------------------------------------------
    ("sum-int-filter.sql", 0): "postgres",  # Postgres SUM over fast field
    ("sum-int-filter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan
    ("sum-numeric15-filter.sql", 0): "postgres",  # Postgres NUMERIC(15,2) sum
    ("sum-numeric15-filter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan
    ("sum-numeric78-filter.sql", 0): "postgres",  # Postgres NUMERIC(78,0) sum
    ("sum-numeric78-filter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan
    # -------------------------------------------------------------------------
    # Other 2-query aggregates (2 files)
    # -------------------------------------------------------------------------
    (
        "aggregate_topk_count.sql",
        0,
    ): "postgres",  # Postgres default GROUP BY + sort baseline
    ("aggregate_topk_count.sql", 1): "aggregate_scan",  # ParadeDB Top-K aggregate scan
    ("bucket-expr-filter.sql", 0): "postgres",  # Postgres date_trunc bucket aggregate
    ("bucket-expr-filter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan
    # -------------------------------------------------------------------------
    # 4-query count and bucket queries (6 files)
    # -------------------------------------------------------------------------
    ("count-filter.sql", 0): "postgres",  # Postgres COUNT(*) fast field
    ("count-filter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan COUNT(*)
    (
        "count-filter.sql",
        2,
    ): "aggregate_scan_count_ctid",  # COUNT(ctid) converted under the hood to pdb.agg
    (
        "count-filter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count direct call with mvcc disabled
    ("count-nofilter.sql", 0): "postgres",  # Postgres COUNT(*) without filter
    ("count-nofilter.sql", 1): "aggregate_scan",  # ParadeDB aggregate scan COUNT(*)
    (
        "count-nofilter.sql",
        2,
    ): "aggregate_scan_count_ctid",  # COUNT(ctid) converted under the hood to pdb.agg
    (
        "count-nofilter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count direct call with mvcc disabled
    (
        "bucket-numeric-filter.sql",
        0,
    ): "postgres",  # Postgres GROUP BY post_type_id numeric fast field
    (
        "bucket-numeric-filter.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan GROUP BY post_type_id
    (
        "bucket-numeric-filter.sql",
        2,
    ): "aggregate_scan_count_col",  # ParadeDB aggregate scan COUNT(post_type_id) with GROUP BY
    (
        "bucket-numeric-filter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count with GROUP BY (mvcc disabled)
    (
        "bucket-numeric-nofilter.sql",
        0,
    ): "postgres",  # Postgres GROUP BY post_type_id without filter
    (
        "bucket-numeric-nofilter.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan GROUP BY post_type_id
    (
        "bucket-numeric-nofilter.sql",
        2,
    ): "aggregate_scan_count_col",  # ParadeDB aggregate scan COUNT(post_type_id) with GROUP BY
    (
        "bucket-numeric-nofilter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count with GROUP BY (mvcc disabled)
    (
        "bucket-string-filter.sql",
        0,
    ): "postgres",  # Postgres GROUP BY badge name string fast field
    (
        "bucket-string-filter.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan GROUP BY name
    (
        "bucket-string-filter.sql",
        2,
    ): "aggregate_scan_count_col",  # ParadeDB aggregate scan COUNT(name) with GROUP BY
    (
        "bucket-string-filter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count with GROUP BY (mvcc disabled)
    (
        "bucket-string-nofilter.sql",
        0,
    ): "postgres",  # Postgres GROUP BY badge name without filter
    (
        "bucket-string-nofilter.sql",
        1,
    ): "aggregate_scan",  # ParadeDB aggregate scan GROUP BY name
    (
        "bucket-string-nofilter.sql",
        2,
    ): "aggregate_scan_count_col",  # ParadeDB aggregate scan COUNT(name) with GROUP BY
    (
        "bucket-string-nofilter.sql",
        3,
    ): "pdb_agg_no_mvcc",  # pdb.agg value_count with GROUP BY (mvcc disabled)
    # -------------------------------------------------------------------------
    # Cardinality (1 file, 10 queries)
    # -------------------------------------------------------------------------
    (
        "cardinality.sql",
        0,
    ): "postgres_distinct",  # COUNT(DISTINCT post_type_id) via Postgres
    (
        "cardinality.sql",
        1,
    ): "postgres_group_by",  # COUNT(*) FROM (SELECT post_type_id ... GROUP BY)
    (
        "cardinality.sql",
        2,
    ): "aggregate_scan_group_by",  # Aggregate scan COUNT(*) GROUP BY subquery
    (
        "cardinality.sql",
        3,
    ): "aggregate_scan_count_col",  # Aggregate scan COUNT(post_type_id) without GROUP BY
    (
        "cardinality.sql",
        4,
    ): "pdb_agg_value_count_no_mvcc",  # pdb.agg value_count without GROUP BY (mvcc disabled)
    (
        "cardinality.sql",
        5,
    ): "postgres_high_cardinality_group_by",  # Postgres multi-agg GROUP BY tags LIMIT 65000
    (
        "cardinality.sql",
        6,
    ): "aggregate_scan_high_cardinality_group_by",  # Aggregate scan multi-agg GROUP BY tags LIMIT 65000
    (
        "cardinality.sql",
        7,
    ): "pdb_agg_high_cardinality_group_by_no_mvcc",  # pdb.agg multi-agg GROUP BY tags (mvcc disabled)
    (
        "cardinality.sql",
        8,
    ): "tantivy_cardinality_mvcc",  # Tantivy cardinality aggregation on tags (mvcc enabled)
    (
        "cardinality.sql",
        9,
    ): "tantivy_cardinality_no_mvcc",  # Tantivy cardinality aggregation on tags (mvcc disabled)
}


def parse_header_comments(raw_content: str) -> tuple[str, str]:
    """Splits a .sql file into (header_comments, query_body).

    File-level markdown headers (Shape, Join, Description, Query Info) are extracted
    for README.md, while any query-specific comments preceding query 0 are preserved
    as part of query_body so they remain attached to query 0's .sql file.
    """
    lines = raw_content.splitlines()
    first_sql_idx = None
    for i, line in enumerate(lines):
        if line.strip() and not line.strip().startswith("--"):
            first_sql_idx = i
            break
    if first_sql_idx is None:
        return raw_content, ""

    comment_lines = lines[:first_sql_idx]
    has_shape = any("Shape:" in l for l in comment_lines)

    if not has_shape:
        # No file header doc block; all comments belong to query 0
        return "", "\n".join(lines)

    # Find where file-level header ends: after the Query Info section
    # (or after the shape/description block if no Query Info exists)
    header_end_idx = first_sql_idx
    in_query_info = False
    for idx, line in enumerate(comment_lines):
        if "Query Info" in line:
            in_query_info = True
        elif in_query_info and not line.strip():
            header_end_idx = idx
            break

    header = "\n".join(lines[:header_end_idx])
    body = "\n".join(lines[header_end_idx:])
    return header, body


def format_readme(header_text: str, fallback_title: str) -> str:
    """Converts the top comment block into a Markdown README."""
    out = []
    in_query_info = False

    for raw_l in header_text.splitlines():
        stripped = raw_l.strip()
        if not stripped:
            if out and out[-1] != "":
                out.append("")
            continue

        content = raw_l.removeprefix("--")
        content = content.removeprefix(" ")

        trimmed = content.strip()
        if trimmed.startswith("Shape:"):
            title = trimmed[len("Shape:") :].strip()
            out.append(f"# {title}\n")
        elif trimmed.startswith("Join:"):
            val = trimmed[len("Join:") :].strip()
            out.append(f"- **Join**: {val}")
        elif trimmed.startswith("Description:"):
            val = trimmed[len("Description:") :].strip()
            out.append(f"- **Description**: {val}")
        elif trimmed.startswith("Note:"):
            val = trimmed[len("Note:") :].strip()
            out.append(f"- **Note**: {val}")
        elif trimmed.startswith("TODO:"):
            val = trimmed[len("TODO:") :].strip()
            out.append(f"- **TODO**: {val}")
        elif trimmed.startswith("Query Info"):
            out.append(f"\n## {trimmed}")
            in_query_info = True
        else:
            if in_query_info:
                out.append(content)
            else:
                out.append(f"  {trimmed}")

    res = "\n".join(out).strip()
    if not res:
        res = f"# {fallback_title}\n"
    elif not res.startswith("#"):
        res = f"# {fallback_title}\n\n" + res

    return res + "\n"


def split_queries(body_text: str) -> list[str]:
    """Splits query body on ;\\n into individual queries."""
    stmts = body_text.split(";\n")
    cleaned = []
    for s in stmts:
        s = s.strip()
        if s:
            if not s.endswith(";"):
                s += ";"
            cleaned.append(s)
    return cleaned


def migrate_file(
    file_path: str,
    output_dir: str,
    dry_run: bool = False,
    remove_original: bool = False,
) -> None:
    filename = os.path.basename(file_path)
    stem = os.path.splitext(filename)[0]
    normalized_stem = stem.replace("-", "_")

    with open(file_path, "r", encoding="utf-8") as f:
        content = f.read()

    header_text, body_text = parse_header_comments(content)
    queries = split_queries(body_text)

    # Do not execute lookups against VARIANT_LOOKUP unless there is more than one query.
    # Single-query files remain flat .sql files, normalized to use underscores.
    if len(queries) <= 1:
        normalized_filename = f"{normalized_stem}.sql"
        if filename != normalized_filename:
            target_path = os.path.join(output_dir, normalized_filename)
            print(
                f"{'[DRY RUN] ' if dry_run else ''}Renaming single-query file `{file_path}` -> `{target_path}`"
            )
            if not dry_run:
                os.rename(file_path, target_path)
        else:
            print(
                f"Skipping single-query file `{file_path}` (already normalized flat .sql file)"
            )
        return

    target_dir = os.path.join(output_dir, normalized_stem)

    variants_to_emit = []
    for idx, q in enumerate(queries):
        key = (filename, idx)
        norm_key = (f"{normalized_stem}.sql", idx)
        name = VARIANT_LOOKUP.get(key)
        if name is None and key not in VARIANT_LOOKUP:
            name = VARIANT_LOOKUP.get(norm_key)
        if (key not in VARIANT_LOOKUP) and (norm_key not in VARIANT_LOOKUP):
            print(
                f"Error: Missing lookup key ({filename}, {idx}) in VARIANT_LOOKUP",
                file=sys.stderr,
            )
            sys.exit(1)
        if name is None:
            print(f"  [{filename}] Dropping variant at index {idx} (mapped to None)")
            continue
        variants_to_emit.append((name, q))

    readme_content = format_readme(header_text, normalized_stem)

    print(
        f"{'[DRY RUN] ' if dry_run else ''}Migrating `{file_path}` -> `{target_dir}/` ({len(variants_to_emit)} variants)"
    )

    if not dry_run:
        os.makedirs(target_dir, exist_ok=True)
        readme_path = os.path.join(target_dir, "README.md")
        with open(readme_path, "w", encoding="utf-8") as f:
            f.write(readme_content)

    for name, q in variants_to_emit:
        sql_filename = f"{name}.sql"
        sql_path = os.path.join(target_dir, sql_filename)
        print(f"    -> {sql_filename}")

        if not dry_run:
            with open(sql_path, "w", encoding="utf-8") as f:
                f.write(q.strip() + "\n")

    if not dry_run and remove_original:
        print(f"  Removing original `{file_path}`")
        os.remove(file_path)


def main():
    parser = argparse.ArgumentParser(
        description="Migrate benchmark SQL queries into nested directory layout."
    )
    parser.add_argument(
        "--dataset-dir",
        default="benchmarks/datasets/stackoverflow",
        help="Path to dataset directory (defaults to benchmarks/datasets/stackoverflow)",
    )
    parser.add_argument(
        "--queries",
        help="Comma-separated query names or glob patterns to migrate (e.g. 'join_disjunctive_local_sort,join_foreign_filter_local_sort')",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print actions without modifying files",
    )
    parser.add_argument(
        "--remove-original",
        action="store_true",
        help="Remove original .sql file after migration",
    )
    args = parser.parse_args()

    queries_dir = os.path.join(args.dataset_dir, "queries")
    if not os.path.isdir(queries_dir):
        print(f"Error: `{queries_dir}` is not a directory.", file=sys.stderr)
        sys.exit(1)

    all_files = sorted(glob.glob(os.path.join(queries_dir, "*.sql")))

    if args.queries:
        patterns = [p.strip() for p in args.queries.split(",")]
        selected = []
        for f in all_files:
            stem = os.path.splitext(os.path.basename(f))[0]
            normalized_stem = stem.replace("-", "_")
            for pat in patterns:
                pat_norm = pat.replace("-", "_")
                if (
                    pat in (stem, normalized_stem)
                    or glob.fnmatch.fnmatch(stem, pat)
                    or glob.fnmatch.fnmatch(normalized_stem, pat_norm)
                    or glob.fnmatch.fnmatch(os.path.basename(f), pat)
                ):
                    selected.append(f)
                    break
    else:
        selected = all_files

    print(f"Found {len(selected)} query files to process.")
    for f in selected:
        migrate_file(
            f, queries_dir, dry_run=args.dry_run, remove_original=args.remove_original
        )


if __name__ == "__main__":
    main()
