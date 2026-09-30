#!/usr/bin/env python3
"""
Fail on any `impl Drop` in pg_search that does not say why it skips `impl_safe_drop!`.

A `Drop` that calls into Postgres has to skip its body while a panic unwinds or the backend
exits (#6479), which is what `impl_safe_drop!` in pg_search/src/postgres/utils.rs does. A bare
`impl Drop` needs a comment directly above it that opts out in so many words: "We
intentionally do NOT use `impl_safe_drop!` here because ...". pg_search is the only crate
that links pgrx, so it is the only one scanned.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
# The comment block and attributes directly above an impl, then its header up to the brace,
# across lines when rustfmt wraps it.
BARE_DROP = re.compile(
    r"((?:^[ \t]*//.*\n)*)(?:^[ \t]*#\[.*\n)*^[ \t]*(impl\b[^{;]*?\bDrop\s+for\s+([^{;]+))",
    re.MULTILINE,
)
# A comment that only names the macro, like a TODO, is not an opt out.
OVERRIDE = re.compile(r"\bnot\s+us(?:e|ing)\W+impl_safe_drop\b", re.IGNORECASE)

# Shapes the matcher must get right, with the findings each should produce.
SELF_TEST = [
    ("impl<T: From<A> + Into<A>> Drop for Nested<'_, T> {", 1),
    ("impl<\n    T: Clone,\n> Drop for Wrapped<T>\n{", 1),
    ("impl std::ops::Drop for Qualified {", 1),
    ("impl DropGuard for NotDrop {", 0),
    ("impl Drop for $ty {", 0),
    ("// TODO: switch this to `impl_safe_drop!`.\nimpl Drop for Todo {", 1),
    (
        "// We intentionally do NOT use `impl_safe_drop!` here.\n\nimpl Drop for Detached {",
        1,
    ),
    (
        "// We intentionally do NOT use `impl_safe_drop!` here.\n#[cfg(test)]\nimpl Drop for A {",
        0,
    ),
]


def find_bare_drops(text):
    """Return (line, header) for every `impl Drop` in `text` without an explaining comment."""
    return [
        (text.count("\n", 0, m.start(2)) + 1, " ".join(m.group(2).split()))
        for m in BARE_DROP.finditer(text)
        # `$ty` is the macro's own expansion.
        if not m.group(3).startswith("$") and not OVERRIDE.search(m.group(1))
    ]


def main():
    """Check the matcher, then report every bare `impl Drop` under pg_search/src."""
    for text, expected in SELF_TEST:
        assert len(find_bare_drops(text)) == expected, text
    bad = [
        f"{path.relative_to(REPO)}:{line}: {header}"
        for path in sorted((REPO / "pg_search" / "src").rglob("*.rs"))
        for line, header in find_bare_drops(path.read_text(encoding="utf-8"))
    ]
    if bad:
        print("\n".join(bad))
        print(
            f"\n{len(bad)} bare `impl Drop`. Use `impl_safe_drop!` from "
            'pg_search/src/postgres/utils.rs, or opt out directly above the impl with "We '
            'intentionally do NOT use `impl_safe_drop!` here because ...".'
        )
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
