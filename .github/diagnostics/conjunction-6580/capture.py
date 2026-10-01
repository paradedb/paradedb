"""Capture query plans after benchmark measurements finish."""

from pathlib import Path

p = Path("benchmarks/Makefile.common")
source = p.read_text(encoding="utf-8")
MARKER = "\nclean: validate\n"
assert source.count(MARKER) == 1
CAPTURE = (
    "\t$(COMPOSE) exec -T paradedb psql -X -U postgres -d benchmark -v ON_ERROR_STOP=1 "
    '< "$$GITHUB_WORKSPACE/.github/diagnostics/conjunction-6580/plans.sql" '
    '> "$$OUT_DIR/plans.txt"\n'
)
source = source.replace(MARKER, CAPTURE + MARKER)
p.write_text(source, encoding="utf-8")
