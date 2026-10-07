#!/usr/bin/env python3
"""Capture complete index-builder expressions so the harness can migrate them."""

import re
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text(encoding="utf-8")
pattern = re.compile(r"^indexing(?=\s*\.paradedbIndex)[\s\S]*?;", re.MULTILINE)
print(
    pattern.sub(
        lambda match: (
            "docsIndexes.push((docsTable) => "
            + re.sub(r"\b(?:mockItems|arrayDemo)\.", "docsTable.", match.group()[:-1])
            + ");"
        ),
        source,
    )
)
