#!/usr/bin/env python3
"""Checks an export against paradedb.vector_clusters and the options the audit assumes."""
import argparse
import json
import struct
import sys


def sizes(path):
    with open(path, "rb") as handle:
        raw = handle.read()
    return list(struct.unpack(f"<{len(raw) // 4}I", raw))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--segments", type=int, required=True)
    args = parser.parse_args()

    with open(f"{args.out}/export/meta.json") as handle:
        meta = json.load(handle)
    with open(f"{args.out}/vector_clusters.json") as handle:
        clusters = json.load(handle)
    failures = []
    if meta["router_kind"] != "quantized_stacked":
        failures.append(f"router kind {meta['router_kind']}")
    if meta["quantization"] != [1, 1]:
        failures.append(f"quantization {meta['quantization']}")
    if meta["spill"] is not False:
        failures.append("spill is on")
    if args.segments == 1:
        if sizes(f"{args.out}/export/sizes.u32") != clusters[meta["segment_id"]]:
            failures.append("cluster sizes differ from paradedb.vector_clusters")
    else:
        if meta["partition_by"] != ["emb"]:
            failures.append(f"partition_by {meta['partition_by']}")
        if len(meta["segments"]) != args.segments:
            failures.append(f"{len(meta['segments'])} exported segments")
        for index, segment in enumerate(meta["segments"]):
            exported = sizes(f"{args.out}/export/segment-{index}/sizes.u32")
            if exported != clusters[segment["segment_id"]]:
                failures.append(f"segment {index}: sizes differ from paradedb.vector_clusters")
            if segment["router_kind"] != "quantized_stacked":
                failures.append(f"segment {index}: router kind {segment['router_kind']}")
            if not segment["partition_centroid"]:
                failures.append(f"segment {index}: no partition centroid")
    if failures:
        sys.exit("\n".join(failures))


if __name__ == "__main__":
    main()
