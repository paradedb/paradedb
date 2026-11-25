#!/usr/bin/env python3
"""
Fail on any `impl Drop` or `DropGuard` in pgrx-linking crates that does not say why it skips
`impl_safe_drop!`, or any direct `IsTransactionState()` call inside `impl_safe_drop!` that does
not say why it skips `ResourceOwnerStamp`.

A `Drop` that calls into Postgres has to skip its body while a panic unwinds or the backend
exits (#6479), which is what `impl_safe_drop!` in common/src/postgres/utils.rs does. A bare
`impl Drop` or `DropGuard::new` needs a comment directly above it that opts out in so many
words: "We intentionally do NOT use `impl_safe_drop!` here because ...".

Inside `impl_safe_drop!`, checking `IsTransactionState()` alone is not enough when a scan state
lives in a `MemoryContext` whose `ResourceOwner` has already been released (#7609); wrappers
should gate cleanup with `ResourceOwnerStamp` from common/src/postgres/resowner.rs, or opt out
directly above the call with "We intentionally call `IsTransactionState()` here because ...".
Every workspace crate that depends on `pgrx` is scanned.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
# The comment block and attributes directly above an impl, then its header up to the brace,
# across lines when rustfmt wraps it.
BARE_DROP = re.compile(
    r"((?:^[ \t]*//.*\n)*)(?:^[ \t]*#\[.*\n)*^[ \t]*(impl\b[^{;]*?\bDrop\s+for\s+([^{;]+))",
    re.MULTILINE,
)
DROP_GUARD = re.compile(r"\bDropGuard::new\b|\bDropGuard\s+as\b")
SAFE_DROP_START = re.compile(r"^[ \t]*(?:[A-Za-z0-9_]+::)*impl_safe_drop!\s*\(")
IS_TX_STATE = re.compile(r"\bIsTransactionState\s*\(")
STRING_LITERAL = re.compile(r'"(?:\\.|[^"\\])*"')
# A comment that only names the macro, like a TODO, is not an opt out.
OVERRIDE = re.compile(r"\bnot\s+us(?:e|ing)\W+impl_safe_drop\b", re.IGNORECASE)
TX_STATE_OVERRIDE = re.compile(
    r"\bnot\s+us(?:e|ing)\W+ResourceOwnerStamp\b|\bintentionally\b.*\bIsTransactionState\b",
    re.IGNORECASE,
)

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
    ("let _g = std::mem::DropGuard::new(x, |_| {});", 1),
    ("use std::mem::DropGuard as Guard;", 1),
    (
        (
            "// We intentionally do NOT use `impl_safe_drop!` here.\n"
            "let _g =\n    std::mem::DropGuard::new(x, |_| {});"
        ),
        0,
    ),
    (
        (
            "// We intentionally do NOT use `impl_safe_drop!` here.\n"
            "let _ok = DropGuard::new(x, |_| {});\n"
            "let _bad =\n    DropGuard::new(y, |_| {});"
        ),
        1,
    ),
    (
        "crate::impl_safe_drop!(Foo, |self| {\n    if unsafe { IsTransactionState() } {}\n});",
        1,
    ),
    (
        (
            "crate::impl_safe_drop!([T] Foo<T>, |self| {\n"
            "    // TODO: switch to ResourceOwnerStamp.\n"
            "    if unsafe { pg_sys::IsTransactionState() } {}\n"
            "});"
        ),
        1,
    ),
    (
        (
            "crate::impl_safe_drop!(Foo, |self| {\n"
            "    // We intentionally call `IsTransactionState()` here because fallback.\n"
            "    if unsafe { IsTransactionState() } {}\n"
            "});"
        ),
        0,
    ),
    (
        (
            "impl_safe_drop!(Foo, |self| {\n"
            "    // We intentionally do not use `ResourceOwnerStamp` here because ...\n"
            "    if unsafe { IsTransactionState() } {}\n"
            "});"
        ),
        0,
    ),
    (
        (
            "impl_safe_drop!(Foo, |self| {\n"
            "    // IsTransactionState() in a comment is fine.\n"
            '    let _s = "IsTransactionState()";\n'
            "});\n"
            "fn outside() { unsafe { IsTransactionState() }; }"
        ),
        0,
    ),
]


def _code_only(line):
    """Strip string literals and `//` comments so paren counting and regexes see code only."""
    return STRING_LITERAL.sub('""', line).split("//", 1)[0]


def _preceding_comment_block(lines, idx):
    """Return the `//` comment block directly above the statement at `idx`."""
    stmt = idx
    while stmt > 0:
        above = lines[stmt - 1].strip()
        if not above or above.startswith("//") or above.endswith((";", "{", "}")):
            break
        stmt -= 1
    while stmt > 0 and lines[stmt - 1].lstrip().startswith("#["):
        stmt -= 1
    comment_start = stmt
    while comment_start > 0 and lines[comment_start - 1].lstrip().startswith("//"):
        comment_start -= 1
    return "\n".join(lines[comment_start:stmt])


def find_bare_drops(text):
    """Return (line, header) for unexcused `impl Drop`, `DropGuard`, or `IsTransactionState()`."""
    findings = [
        (text.count("\n", 0, m.start(2)) + 1, " ".join(m.group(2).split()))
        for m in BARE_DROP.finditer(text)
        # `$ty` is the macro's own expansion.
        if not m.group(3).startswith("$") and not OVERRIDE.search(m.group(1))
    ]
    lines = text.splitlines()
    safe_drop_paren_depth = 0
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if DROP_GUARD.search(stripped) and not OVERRIDE.search(
            _preceding_comment_block(lines, idx)
        ):
            findings.append((idx + 1, stripped))

        code = _code_only(line)
        if safe_drop_paren_depth == 0:
            m = SAFE_DROP_START.match(code)
            if m:
                rest = code[m.end() - 1 :]
                safe_drop_paren_depth = rest.count("(") - rest.count(")")
                if IS_TX_STATE.search(rest) and not TX_STATE_OVERRIDE.search(
                    _preceding_comment_block(lines, idx)
                ):
                    findings.append((idx + 1, stripped))
        else:
            if IS_TX_STATE.search(code) and not TX_STATE_OVERRIDE.search(
                _preceding_comment_block(lines, idx)
            ):
                findings.append((idx + 1, stripped))
            safe_drop_paren_depth = max(
                0, safe_drop_paren_depth + code.count("(") - code.count(")")
            )
    return sorted(findings)


def pgrx_crate_src_dirs():
    """Return `src/` directories for all workspace crates that depend on `pgrx`."""
    out = subprocess.check_output(
        [
            "cargo",
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            str(REPO / "Cargo.toml"),
        ],
        text=True,
    )
    meta = json.loads(out)
    return sorted(
        Path(pkg["manifest_path"]).parent / "src"
        for pkg in meta["packages"]
        if any(dep["name"] == "pgrx" for dep in pkg["dependencies"])
    )


def main():
    """Check the matcher, then report every bare `impl Drop` across pgrx-linking crates."""
    for text, expected in SELF_TEST:
        assert len(find_bare_drops(text)) == expected, text
    bad = [
        f"{path.relative_to(REPO)}:{line}: {header}"
        for src_dir in pgrx_crate_src_dirs()
        for path in sorted(src_dir.rglob("*.rs"))
        for line, header in find_bare_drops(path.read_text(encoding="utf-8"))
    ]
    if bad:
        print("\n".join(bad))
        print(
            f"\n{len(bad)} drop-safety violation(s). Use `impl_safe_drop!` from "
            'common/src/postgres/utils.rs (or opt out directly above with "We intentionally do '
            'NOT use `impl_safe_drop!` here because ..."), and inside `impl_safe_drop!` gate C '
            "cleanup with `ResourceOwnerStamp` from common/src/postgres/resowner.rs rather than "
            'calling `IsTransactionState()` directly (or opt out directly above with "We '
            'intentionally call `IsTransactionState()` here because ...").'
        )
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
