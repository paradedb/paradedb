#!/usr/bin/env python3
"""Plot saved results without rebuilding either index."""

import argparse
import json

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.ticker import PercentFormatter

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("results")
parser.add_argument(
    "--output", help="Chart directory; defaults to the results directory"
)
args = parser.parse_args()
from pathlib import Path

root = Path(args.results)
output = Path(args.output) if args.output else root
output.mkdir(parents=True, exist_ok=True)
manifest = json.loads((root / "manifest.json").read_text())
summary = json.loads((root / "summary.json").read_text())
curves = np.load(root / "curves.npz")

fig, axes = plt.subplots(1, 2, figsize=(11, 4.7), constrained_layout=True)
for engine, label, color in [
    ("pg_search", "pg_search", "#176caa"),
    ("vectorsearch", "pg_vectorsearch / PRISM", "#d07828"),
]:
    axes[0].plot(
        curves[f"{engine}_vectors_scanned"] / manifest["rows"],
        curves[f"{engine}_recall"],
        label=label,
        color=color,
        linewidth=2,
    )
    data = np.load(root / f"{engine}-clusters.npz")
    sizes = np.sort(np.bincount(data["labels"], minlength=len(data["centroids"])))
    axes[1].plot(
        sizes,
        np.arange(1, len(sizes) + 1) / len(sizes),
        label=f"{label} ({len(sizes):,} clusters)",
        color=color,
        linewidth=2,
    )

axes[0].set(
    xlim=(0, 0.1),
    ylim=(0, 1.005),
    xlabel="Mean fraction of posting rows scanned",
    ylabel="Recall@10",
    title="Exact centroid order, stored memberships",
)
axes[0].xaxis.set_major_formatter(PercentFormatter(1))
axes[0].yaxis.set_major_formatter(PercentFormatter(1))
axes[1].set(
    xlabel="Rows per cluster (log scale)",
    ylabel="Fraction of clusters at or below size",
    title="Cluster size distribution",
    ylim=(0, 1.01),
)
axes[1].set_xscale("symlog", linthresh=1)
axes[1].set_xlim(left=0)
axes[1].yaxis.set_major_formatter(PercentFormatter(1))
for ax in axes:
    ax.grid(alpha=0.2)
    ax.legend(loc="lower right", frameon=False, fontsize=9)
    ax.spines[["top", "right"]].set_visible(False)
dataset = manifest.get("dataset_name", "Cohere")
configuration = (
    "Native index defaults · replica postings included · one build each"
    if "index_configuration" in manifest
    else "Full training set · no replication · PRISM leaf refinement off · one build each"
)
if manifest.get("target_segment_count") is not None:
    configuration = (
        f"pg_search target {manifest['target_segment_count']} segments · other index defaults · replicas included"
    )
fig.suptitle(
    f"Actual extension exports · {manifest['rows']:,} {dataset} vectors · {manifest['queries']} queries\n"
    f"{configuration}",
    fontsize=12,
)
fig.savefig(output / "comparison.png", dpi=180)
fig.savefig(output / "comparison.svg")
