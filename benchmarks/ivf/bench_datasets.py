#!/usr/bin/env python3
"""
Shared dataset + config plumbing for the IVF sweep/profiler scripts.

Two responsibilities:

1. DATASET REGISTRY -- map a (dataset, size) pair to a table name and the
   on-disk test/ground-truth parquet paths, so the sweep and the profiler
   resolve variants identically. One family:

     cohere : VectorDBBench Cohere, 768d cosine. Sizes 100000 / 1000000 / ...
              Paths come from vectordb_bench's Dataset.COHERE manager.

   (SIFT was removed -- cohere is the target benchmark. To re-add a family,
   write a `_resolve_<name>` and register it in `_RESOLVERS`; the Dataset
   carries dim/metric/column-names so everything downstream is unchanged.)

2. CONFIG MERGE -- load a JSON file of the same keys the CLI exposes and merge
   it UNDER argparse, so an explicit flag always wins over the file, the file
   wins over the built-in default. This keeps the long invocations
   (50GB-buffer 1M runs with custom fanout/K grids) in a checked-in file
   instead of a shell-history one-liner.

A config file is just the long-form flags as JSON, e.g.:

    {
      "size": 1000000,
      "filters": "none,sel1",
      "probe_fractions": "0.01,0.02,0.05,0.1,0.2",
      "ks": "1,10",
      "n_q": 1000
    }
"""

import argparse
import json


# ----------------------------------------------------------------- datasets

def numerize(n: int) -> str:
    if n % 1_000_000 == 0:
        return f"{n // 1_000_000}m"
    if n % 1_000 == 0:
        return f"{n // 1_000}k"
    return str(n)


class Dataset:
    """Resolved dataset variant: table name, parquet paths, geometry, and the
    parquet column names the loaders should read."""

    def __init__(self, family, size, table, test_path, gt_path,
                 dim, metric, vec_col, qid_col, gt_col):
        self.family = family
        self.size = size
        self.table = table
        self.test_path = test_path
        self.gt_path = gt_path
        self.dim = dim
        self.metric = metric          # "cosine" | "l2"  (informational/sanity)
        self.vec_col = vec_col        # query-vector column in the test parquet
        self.qid_col = qid_col        # query-id column in the test parquet
        self.gt_col = gt_col          # neighbor-id list column in the gt parquet

    @property
    def dist_op(self):
        """pgvector distance operator matching the metric: <-> for L2,
        <=> for cosine. The harness ORDER BY must use the operator the
        index opclass was built with, or the scan won't push down."""
        return "<->" if self.metric == "l2" else "<=>"

    def __str__(self):
        return (f"{self.family}/{numerize(self.size)} table={self.table} "
                f"dim={self.dim} metric={self.metric}")


def _resolve_cohere(size, table, test, gt):
    """Cohere via vectordb_bench. test/gt override the resolved paths."""
    if test is None or gt is None:
        try:
            from vectordb_bench.backend.dataset import Dataset as VDBDataset
            m = VDBDataset.COHERE.manager(size)
            if test is None:
                test = str(m.data_dir / m.data.test_file)
            if gt is None:
                try:
                    from vectordb_bench.backend.filter import non_filter
                    gt = str(m.data_dir / non_filter.groundtruth_file)
                except Exception:
                    hits = list(m.data_dir.glob("*neighbor*.parquet"))
                    gt = str(hits[0]) if hits else None
        except Exception:
            pass  # leave as None; caller validates
    return Dataset(
        family="cohere", size=size,
        table=table or f"cohere_{numerize(size)}",
        test_path=test, gt_path=gt,
        dim=768, metric="cosine",
        vec_col="emb", qid_col="id", gt_col="neighbors_id",
    )


_RESOLVERS = {"cohere": _resolve_cohere}


def resolve_dataset(dataset, size, table=None, test=None, gt=None):
    """Return a Dataset for (dataset family, size), honoring explicit
    table/test/gt overrides. Raises if the family is unknown."""
    family = dataset.lower()
    if family not in _RESOLVERS:
        raise SystemExit(f"unknown dataset '{dataset}' "
                         f"(choose from {sorted(_RESOLVERS)})")
    return _RESOLVERS[family](size, table, test, gt)


# ------------------------------------------------------------------- config

def load_config_into(args, config_path, known_keys):
    """Merge a JSON config UNDER already-parsed argparse `args`: file value is
    applied only where the user did NOT pass the flag (i.e. the arg still holds
    its default). Returns the set of keys taken from the file, for logging.

    `known_keys` maps json_key -> (dest, default) so we can tell whether the
    parsed value is still the default (=> file may override) or user-supplied
    (=> file is ignored for that key). dest names use argparse underscores.
    """
    with open(config_path) as f:
        cfg = json.load(f)
    applied = []
    for key, val in cfg.items():
        if key.startswith("_"):      # _comment etc. -- JSON has no comments
            continue
        dest = key.replace("-", "_")
        if dest not in known_keys:
            print(f"[config] WARNING: ignoring unknown key '{key}'", flush=True)
            continue
        default = known_keys[dest]
        if getattr(args, dest) == default:      # user didn't override on CLI
            setattr(args, dest, val)
            applied.append(key)
    return cfg, applied


def _flatten_spec(spec):
    """Normalize every accepted list-param shape to a flat list of stripped,
    non-empty string tokens. Accepts: a scalar (int/float/str), a comma
    string ("a,b" or "a, b"), a JSON list of scalars, or a JSON list whose
    elements are themselves comma strings (["a, b"]) -- all equivalent."""
    items = spec if isinstance(spec, (list, tuple)) else [spec]
    out = []
    for item in items:
        for tok in str(item).split(","):
            tok = tok.strip()
            if tok:
                out.append(tok)
    return out


def parse_int_list(spec):
    """'1,10,100' / [1,10] / ["1, 10"] / 1 -> flat [int]."""
    toks = _flatten_spec(spec)
    try:
        return [int(t) for t in toks]
    except ValueError as e:
        raise SystemExit(f"bad int list {spec!r}: {e}")


def parse_float_list(spec):
    toks = _flatten_spec(spec)
    try:
        return [float(t) for t in toks]
    except ValueError as e:
        raise SystemExit(f"bad float list {spec!r}: {e}")


def parse_str_list(spec):
    return _flatten_spec(spec)