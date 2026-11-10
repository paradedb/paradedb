"""Use the search profile and index SQL for a heap-only pdbench-load finalize run."""

import argparse
import json
import shutil
from pathlib import Path

import tomllib


def prepare(config_path, assets, pdbench, output):
    """Write a finalize config using the profile's existing DDL and reloptions."""
    config = tomllib.loads(config_path.read_text())
    if config.get("engine", "pg_search") != "pg_search":
        raise ValueError("Expected a pg_search benchmark profile")
    dataset = config["dataset"]
    name = dataset["name"]
    table = dataset["table"]
    directory = output.resolve() / "dataset"
    scripts = directory / "paradedb"
    scripts.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(
        pdbench / "datasets" / name / "schema.yaml", directory / "schema.yaml"
    )
    shutil.copyfile(assets / name / "create_table.sql", scripts / "pre.sql")
    (scripts / "post.sql").write_text(
        "DROP INDEX IF EXISTS {{ index_name }};\n"
        + (assets / name / "index.sql").read_text()
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
            "type": "paradedb",
            "url": "${PARADEDB_URL}",
            "container": "${PARADEDB_CONTAINER}",
            "dataset": str(directory),
            "table": table,
            "settings": config["postgres"]["build"],
            "vars": {
                "index_name": f"{table}_bm25_idx",
                "with_options": ", ".join(options),
            },
        },
        "output": str(output.resolve() / "runs"),
    }
    path = output / "load.json"
    path.write_text(json.dumps(load, indent=2) + "\n")
    return path


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
