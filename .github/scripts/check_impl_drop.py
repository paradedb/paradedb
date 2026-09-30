#!/usr/bin/env python3
"""
Fail on any bare `impl Drop` in pg_search that does not say why it skips `impl_safe_drop!`.

A `Drop` that touches Postgres state has to skip its body while a panic is unwinding or the
backend is exiting, or it can raise a second error and abort the backend (#6479). The
`impl_safe_drop!` macro in pg_search/src/postgres/utils.rs does that. A bare `impl Drop` is
allowed only when the comment block directly above it names `impl_safe_drop!`, and that
comment is where to say why the macro does not apply.

pg_search is the only crate that links pgrx, so it is the only one scanned.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SRC = REPO / "pg_search" / "src"
# From `impl` to the opening brace, across lines when rustfmt wraps a long header.
IMPL_DROP = re.compile(r"^[ \t]*(impl\b[^{;]*?\bDrop\s+for\s+([^{;]+))", re.MULTILINE)
OVERRIDE = "impl_safe_drop"

# Snippets the matcher must classify correctly: text, expected finding count.
SELF_TEST = [
    ("impl Drop for Bare {", 1),
    ("    impl Drop for Indented {", 1),
    ("impl std::ops::Drop for Qualified {", 1),
    (
        "impl<T: From<PgItem> + Into<PgItem> + Debug + Clone> Drop for AtomicGuard<'_, T> {",
        1,
    ),
    ("impl<\n    T: Clone,\n> Drop for Wrapped<T>\n{", 1),
    ("impl DropGuard for NotDrop {", 0),
    ("crate::impl_safe_drop!(Safe, |self| {", 0),
    ("impl Drop for $ty {", 0),
    (
        "// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body\n"
        "// must run while unwinding.\n"
        "impl Drop for Explained {",
        0,
    ),
    ("// NOTE: `impl_safe_drop!` does not apply.\n\nimpl Drop for Detached {", 1),
    (
        "// NOTE: `impl_safe_drop!` does not apply.\n#[cfg(test)]\nimpl<T> Drop for Attr<T> {",
        0,
    ),
    ("// unrelated comment\nimpl Drop for Unexplained {", 1),
]


def find_bare_drops(text):
    """Return (line_number, header) for every unexplained `impl Drop` in `text`."""
    lines = text.splitlines()
    found = []
    for match in IMPL_DROP.finditer(text):
        if match.group(2).startswith("$"):
            # `$ty` is the macro's own expansion in utils.rs.
            continue
        i = text.count("\n", 0, match.start())
        # The explanation is the comment block directly above the impl; attributes may
        # sit between the two.
        j = i - 1
        while j >= 0 and lines[j].lstrip().startswith("#["):
            j -= 1
        comment = []
        while j >= 0 and lines[j].lstrip().startswith("//"):
            comment.append(lines[j])
            j -= 1
        if not any(OVERRIDE in line for line in comment):
            found.append((i + 1, " ".join(match.group(1).split())))
    return found


def main():
    """Check the matcher, then report every bare `impl Drop` under pg_search/src."""
    for text, expected in SELF_TEST:
        if len(find_bare_drops(text)) != expected:
            print(
                f"matcher self test failed on {text!r}: expected {expected} finding(s)"
            )
            return 1
    bad = []
    for path in sorted(SRC.rglob("*.rs")):
        for line, header in find_bare_drops(path.read_text(encoding="utf-8")):
            bad.append(f"{path.relative_to(REPO)}:{line}: bare `{header}`")
    if bad:
        print("\n".join(bad))
        print(
            f"\n{len(bad)} bare `impl Drop`. Use `impl_safe_drop!` from "
            "pg_search/src/postgres/utils.rs, or explain why it does not apply in a comment "
            "directly above the impl that names `impl_safe_drop!`."
        )
        return 1
    print("no bare impl Drop")
    return 0


if __name__ == "__main__":
    sys.exit(main())
