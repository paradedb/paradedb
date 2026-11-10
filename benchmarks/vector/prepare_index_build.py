"""Use the search profile and index SQL for a heap-only pdbench-load finalize run."""

import argparse
import json
import shutil
from pathlib import Path

import tomllib


def prepare(config_path, assets, pdbench, output):
    """Write a finalize config using the profile's existing DDL and reloptions."""
    config = tomllib.loads(config_path.read_text())
    backend, suffix = {
        "pg_search": ("paradedb", "bm25"),
        "pg_vectorsearch": ("timescale", "prism"),
    }[config.get("engine", "pg_search")]
    dataset = config["dataset"]
    directory = output.resolve() / "dataset"
    scripts = directory / backend
    scripts.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(
        pdbench / "datasets" / dataset["name"] / "schema.yaml",
        directory / "schema.yaml",
    )
    shutil.copyfile(assets / dataset["name"] / "create_table.sql", scripts / "pre.sql")
    index_assets = assets / dataset["name"]
    if backend == "timescale":
        index_assets /= "pg_vectorsearch"
    (scripts / "post.sql").write_text(
        "DROP INDEX IF EXISTS {{ index_name }};\n"
        + (index_assets / "index.sql").read_text()
    )
    options = []
    for key, value in config["index"].items():
        if isinstance(value, str):
            value = "'" + value.replace("'", "''") + "'"
        elif isinstance(value, bool):
            value = str(value).lower()
        elif not isinstance(value, (int, float)):
            raise TypeError(f"Unsupported index option: {key}")
        options.append(f"{key}={value}")
    load = {
        "datasource": {"path": dataset["s3_base_path"]},
        "datastore": {
            "type": backend,
            "url": "${PARADEDB_URL}",
            "container": "${PARADEDB_CONTAINER}",
            "dataset": str(directory),
            "table": dataset["table"],
            "settings": config["postgres"]["build"],
            "vars": {
                "index_name": f"{dataset['table']}_{suffix}_idx",
                "with_options": ", ".join(options),
            },
        },
        "output": str(output.resolve() / "runs"),
    }
    (output / "load.json").write_text(json.dumps(load, indent=2) + "\n")
    return output / "load.json"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--pdbench", type=Path, default=Path("pdbench"))
    parser.add_argument(
        "--output", type=Path, default=Path("benchmark-artifacts/index-build")
    )
    args = parser.parse_args()
    prepare(args.config, args.assets, args.pdbench, args.output)
