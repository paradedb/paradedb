#!/usr/bin/env python3
"""Run the documented setup, migration, first index and query in fresh databases.

The individual snippet harnesses supply working connections, models, tables and
indexes. They can pass even when the documented installation, configuration or
migrations are broken. This runner covers that gap by executing each tutorial's
actual setup instructions before checking its first index and query results.

File contents and commands come from the docs, not the shared snippet models.
Interactive REPL launch commands run headlessly; only connection placeholders,
settings ellipses and the indicated migration insertion points are substituted.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path

from extract_code_snippets import (
    CODEGROUP_PATTERN,
    COVERAGE_PATH,
    DOCS_ROOT,
    FENCE_PATTERN,
    inventory,
    resolve_coverage,
)

TEXT = (DOCS_ROOT / "start/configure-your-environment.mdx").read_text()
TABS = dict(re.findall(r'<Tab title="([^"\n]+)">(.*?)</Tab>', TEXT, re.DOTALL))
ENV = os.environ.copy()
CONSUMED_FENCES = set()
LABELS = {
    "sql": "SQL",
    "drizzle": "Drizzle",
    "django": "Django",
    "sqlalchemy": "SQLAlchemy",
    "rails": "Rails",
    "efcore": "EF Core",
}


def run(command, cwd, env, source=None):
    """Execute a documented command and propagate failures."""
    subprocess.run(command, cwd=cwd, env=env, input=source, text=True, check=True)


def block(target, info=None, index=0):
    """Read a numbered fence from a getting-started language tab."""
    fences = FENCE_PATTERN.findall(TABS[LABELS[target]])
    if info is not None:
        fences = [(label, body) for label, body in fences if label == info]
    info, body = fences[index]
    CONSUMED_FENCES.add(hashlib.sha256((info + "\n" + body).encode()).hexdigest())
    return body


def example(page, target):
    """Read the target example from the first application CodeGroup."""
    group = CODEGROUP_PATTERN.search((DOCS_ROOT / page).read_text()).group()
    return next(
        body
        for info, body in FENCE_PATTERN.findall(group)
        if LABELS[target].lower() in info.lower()
    )


def connection_text(text, env):
    """Replace documented connection placeholders with the isolated database."""
    user = env["PGUSER"]
    password = env.get("PGPASSWORD", "")
    host, port, database = env["PGHOST"], env["PGPORT"], env["PGDATABASE"]
    uri = f"postgres://{user}:{password}@{host}:{port}/{database}"
    text = text.replace("postgres://postgres:<PASSWORD>@localhost:5432/paradedb", uri)
    text = text.replace(
        "postgresql+psycopg://postgres:<PASSWORD>@localhost:5432/paradedb",
        uri.replace("postgres://", "postgresql+psycopg://"),
    )
    text = text.replace(
        "Host=localhost;Port=5432;Database=paradedb;Username=postgres;Password=<PASSWORD>",
        f"Host={host};Port={port};Database={database};Username={user};Password={password}",
    )
    for old, new in (
        ("<PASSWORD>", password),
        ('"paradedb"', f'"{database}"'),
        ('"postgres"', f'"{user}"'),
        ('"localhost"', f'"{host}"'),
        ('"5432"', f'"{port}"'),
    ):
        text = text.replace(old, new)
    return text


def shell(source, cwd, env, prefix=""):
    """Run a documented command block without an interactive shell."""
    run(["bash", "-e", "-c", prefix + "\n" + source], cwd, env)


def python_env(cwd, env):
    """Activate the Python environment created by the documentation."""
    env = env.copy()
    env["PATH"] = str(cwd / ".venv/bin") + os.pathsep + env["PATH"]
    return env


def verify(cwd, env):
    # Validate real data and schema produced by the documented migrations/index command.
    """Assert that the documented setup produced demo data and a valid index."""
    query = """DO $$ BEGIN
    IF (SELECT count(*) FROM mock_items) = 0 THEN RAISE EXCEPTION 'Demo table is empty'; END IF;
    IF NOT EXISTS (SELECT 1 FROM mock_items
      WHERE description = 'Sleek running shoes' AND rating = 5 AND category = 'Footwear')
      THEN RAISE EXCEPTION 'Expected demo row missing'; END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_index
      WHERE indexrelid = to_regclass('search_idx') AND indisvalid AND indisready)
      THEN RAISE EXCEPTION 'First index missing or invalid'; END IF;
    END $$;"""
    run(["psql", "-v", "ON_ERROR_STOP=1", "-c", query], cwd, env)


def setup_sql(cwd, env):
    """Execute the SQL tutorial against a fresh database."""
    for source in [
        block("sql", "sql", 0),
        block("sql", "sql", 1),
        example("start/create-your-first-index.mdx", "sql"),
    ]:
        run(["psql", "-v", "ON_ERROR_STOP=1"], cwd, env, source)

    result = subprocess.run(
        ["psql", "-v", "ON_ERROR_STOP=1", "-At"],
        cwd=cwd,
        env=env,
        input=example("start/run-queries.mdx", "sql"),
        text=True,
        capture_output=True,
        check=True,
    )
    assert [line.split("|")[0] for line in result.stdout.splitlines()] == [
        "White jogging shoes",
        "Generic shoes",
        "Sleek running shoes",
    ]


def setup_drizzle(cwd, env):
    """Execute the Drizzle tutorial and assert the expected first query rows."""
    shell(block("drizzle", "bash", 0), cwd, env)
    (cwd / "db.ts").write_text(connection_text(block("drizzle", "ts db.ts"), env))
    source = block("drizzle", "ts", 0) + "\n" + block("drizzle", "ts", 1)
    for page in ("start/create-your-first-index.mdx", "start/run-queries.mdx"):
        # Static imports are already present as the documented dynamic REPL bindings.
        body = re.sub(
            r"^import .*?;\n",
            "",
            example(page, "drizzle"),
            flags=re.MULTILINE | re.DOTALL,
        )
        if page == "start/run-queries.mdx":
            body = body.replace("await db", "const queryRows = await db", 1)
        source += "\n" + body
    source += (
        "\nif (JSON.stringify(queryRows.map(row => row.description)) !== "
        'JSON.stringify(["White jogging shoes", "Generic shoes", "Sleek '
        'running shoes"])) throw new Error("Unexpected tutorial query '
        'rows");\nawait client.end();\n'
    )
    (cwd / "setup.ts").write_text(source)
    run(["npx", "tsx", "setup.ts"], cwd, env)


def setup_django(cwd, env):
    """Execute the Django migrations and tutorial query."""
    shell(block("django", "bash", 0), cwd, env)
    env = python_env(cwd, env)
    # Merge the settings snippet's indicated existing INSTALLED_APPS, rather than
    # treating the literal ellipsis as a Python app entry.
    settings = connection_text(block("django", "python myproject/settings.py"), env)
    settings = settings.replace("INSTALLED_APPS = [\n    ...,", "INSTALLED_APPS += [")
    with (cwd / "myproject/settings.py").open("a") as file:
        file.write("\n" + settings)
    (cwd / "myapp/models.py").write_text(block("django", "python models.py"))
    shell(block("django", "bash", 1), cwd, env)
    source = (
        block("django", "python")
        + "\nfrom myapp.models import MockItem\nfrom paradedb import MatchAny, ParadeDB\n"
        + example("start/create-your-first-index.mdx", "django")
        + "\n"
        + "tutorial_rows = "
        + example("start/run-queries.mdx", "django").split("\n\n", 1)[1]
        + (
            '\nassert [row["description"] for row in tutorial_rows] == ["White '
            'jogging shoes", "Generic shoes", "Sleek running shoes"]\n'
        )
    )
    run(["python3", "manage.py", "shell", "-c", source], cwd, env)


def setup_sqlalchemy(cwd, env):
    """Execute the Alembic migration and SQLAlchemy tutorial query."""
    shell(block("sqlalchemy", "bash", 0), cwd, env)
    env = python_env(cwd, env)
    shell(block("sqlalchemy", "bash", 1), cwd, env)
    ini = cwd / "alembic.ini"
    ini.write_text(
        re.sub(
            r"^sqlalchemy.url = .*",
            connection_text(block("sqlalchemy", "ini alembic.ini"), env).strip(),
            ini.read_text(),
            flags=re.MULTILINE,
        )
    )
    (cwd / "models.py").write_text(block("sqlalchemy", "python", 0))
    (cwd / "migrations/env.py").write_text(
        block("sqlalchemy", "python migrations/env.py")
    )
    shell(block("sqlalchemy", "bash", 2), cwd, env)
    migration = next((cwd / "migrations/versions").glob("0001_*.py"))
    source = migration.read_text()
    source = source[: source.index("def upgrade()")] + block("sqlalchemy", "python", 1)
    migration.write_text(source)
    shell(block("sqlalchemy", "bash", 3), cwd, env)
    source = (
        connection_text(block("sqlalchemy", "python", 2), env)
        + "\n"
        + example("start/create-your-first-index.mdx", "sqlalchemy")
        + "\n"
        + example("start/run-queries.mdx", "sqlalchemy").replace(
            "session.execute(stmt).all()", "tutorial_rows = session.execute(stmt).all()"
        )
        + (
            '\nassert [row.description for row in tutorial_rows] == ["White '
            'jogging shoes", "Generic shoes", "Sleek running shoes"]\n'
        )
    )
    run(["python3", "-c", source], cwd, env)


def setup_rails(cwd, env):
    """Generate a Rails app and execute its documented migration and query."""
    env = env.copy()
    env["GEM_HOME"] = str(cwd / "gems")
    env["GEM_PATH"] = env["GEM_HOME"]
    env["PATH"] = str(cwd / "gems/bin") + os.pathsep + env["PATH"]
    # A fresh GEM_HOME needs its own bundle executable, even when Ruby ships
    # Bundler as a default gem without a launcher on the runner PATH.
    run(["gem", "install", "bundler", "--no-document"], cwd, env)
    run(["gem", "install", "rails", "--version", "~> 8.1", "--no-document"], cwd, env)
    shell(block("rails", "bash", 0), cwd, env)
    app = cwd / "paradedb"
    with (app / "Gemfile").open("a") as file:
        file.write("\n" + block("rails", "ruby Gemfile"))
    shell(block("rails", "bash", 1), app, env)
    config = block("rails", "yml config/database.yml")
    for key, value in (
        ("database", env["PGDATABASE"]),
        ("username", env["PGUSER"]),
        ("password", env.get("PGPASSWORD", "")),
        ("host", env["PGHOST"]),
        ("port", env["PGPORT"]),
    ):
        config = re.sub(rf"  {key}: .*", f"  {key}: {value}", config)
    (app / "config/database.yml").write_text(config)
    shell(block("rails", "bash", 2), app, env)
    migration = next((app / "db/migrate").glob("*_create_mock_items_table.rb"))
    source = migration.read_text()
    migration.write_text(
        source[: source.index("  def change")]
        + block("rails", "ruby db/migrate/*_create_mock_items_table.rb")
        + "\nend\n"
    )
    (app / "app/models/mock_item.rb").write_text(
        block("rails", "ruby app/models/mock_item.rb")
    )
    shell(block("rails", "bash", 3), app, env)
    source = (
        example("start/create-your-first-index.mdx", "rails")
        + "\n"
        + "tutorial_rows = "
        + example("start/run-queries.mdx", "rails")
        + (
            '\nraise "Unexpected tutorial rows" unless '
            'tutorial_rows.map(&:description) == ["White jogging shoes", '
            '"Generic shoes", "Sleek running shoes"]\n'
        )
    )
    run(["bundle", "exec", "rails", "runner", source], app, env)


def setup_efcore(cwd, env):
    """Execute EF Core migration creation, seed insertion and tutorial query."""
    shell(block("efcore", "bash", 0), cwd, env)
    (cwd / "Program.cs").write_text(
        connection_text(block("efcore", "cs Program.cs"), env)
    )
    shell(block("efcore", "bash", 1), cwd, env)
    migration = next((cwd / "Migrations").glob("*_CreateMockItems.cs"))
    source = migration.read_text()
    # Insert exactly where the prose requests: at the end of Up, before Down.
    end = re.search(
        r"^        }",
        source[source.index("protected override void Up") :],
        re.MULTILINE,
    ).start() + source.index("protected override void Up")
    source = source[:end] + block("efcore", "cs", 1) + "\n" + source[end:]
    migration.write_text(source)
    shell(block("efcore", "bash", 2), cwd, env)
    program = cwd / "Program.cs"
    source = program.read_text().replace(
        "// Replace this with the query you want to run.",
        example("start/create-your-first-index.mdx", "efcore"),
    )
    source = source.replace(
        "PrintResults(results);",
        (
            "if (!results.Select(row => row.Description).SequenceEqual(new[] {"
            ' "White jogging shoes", "Generic shoes", "Sleek running shoes" '
            '})) throw new InvalidOperationException("Unexpected tutorial '
            'rows");\nPrintResults(results);'
        ),
    )
    program.write_text(source)
    shell(block("efcore", "bash", 3), cwd, env)


def validate_setup_coverage(target):
    """Reject setup inventory entries that the scenario did not actually consume."""
    _, outside = inventory()
    tab_digests = {
        hashlib.sha256((info + "\n" + body).encode()).hexdigest()
        for info, body in FENCE_PATTERN.findall(TABS[LABELS[target]])
    }
    missing = [
        key
        for key, entry in resolve_coverage(
            outside, json.loads(COVERAGE_PATH.read_text())
        ).items()
        if entry["mode"] == "setup"
        and outside[key]["sha256"] in tab_digests
        and outside[key]["sha256"] not in CONSUMED_FENCES
    ]
    if missing:
        raise ValueError(f"Setup fences were not executed: {missing}")


def main():
    """Run each requested tutorial in a disposable database."""
    targets = sys.argv[1:] or list(LABELS)
    for target in targets:
        CONSUMED_FENCES.clear()
        if target not in LABELS:
            raise ValueError(f"Unknown target {target}")
        database = "docs_setup_" + uuid.uuid4().hex[:12]
        env = ENV.copy()
        env.setdefault("PGHOST", "localhost")
        env.setdefault("PGPORT", "28818")
        env.setdefault("PGUSER", os.environ.get("USER", "postgres"))
        env.setdefault("PGDATABASE", "postgres")
        # Dependencies follow the docs' commands; optional unused npm peers are ignored.
        env["npm_config_legacy_peer_deps"] = "true"
        with tempfile.TemporaryDirectory(prefix=f"docs-setup-{target}-") as directory:
            cwd = Path(directory)
            run(
                [
                    "psql",
                    "-v",
                    "ON_ERROR_STOP=1",
                    "-c",
                    (
                        f"CREATE DATABASE {database} TEMPLATE template0 "
                        "ENCODING 'UTF8' LC_COLLATE 'C' LC_CTYPE 'C'"
                    ),
                ],
                cwd,
                env,
            )
            try:
                env["PGDATABASE"] = database
                env["DATABASE_URL"] = (
                    f"postgres://{env['PGUSER']}:{env.get('PGPASSWORD', '')}"
                    f"@{env['PGHOST']}:{env['PGPORT']}/{database}"
                )
                run(
                    [
                        "psql",
                        "-v",
                        "ON_ERROR_STOP=1",
                        "-c",
                        "CREATE EXTENSION vector; CREATE EXTENSION pg_search;",
                    ],
                    cwd,
                    env,
                )
                print(
                    f"Testing documented {LABELS[target]} setup in {database}",
                    flush=True,
                )
                globals()["setup_" + target](cwd, env)
                validate_setup_coverage(target)
                verify(cwd, env)
                print(
                    f"PASS: {LABELS[target]} setup, migrations, first index and first query",
                    flush=True,
                )
            finally:
                env["PGDATABASE"] = ENV.get("PGDATABASE", "postgres")
                run(
                    [
                        "psql",
                        "-v",
                        "ON_ERROR_STOP=1",
                        "-c",
                        f"DROP DATABASE {database} WITH (FORCE)",
                    ],
                    cwd,
                    env,
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
