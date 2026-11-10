#!/usr/bin/env python3
"""Bound per-commit HTML reports; preserve aggregate benchmark history."""
import argparse
import json
import shutil
import time
from pathlib import Path


def prune(root, now=None, max_age_days=30, max_bytes=512 * 1024 * 1024):
    reports = root / 'benchmarker-runs'
    if not reports.exists():
        return
    if reports.is_symlink():
        raise ValueError('Report root must not be a symlink')
    # Use recorded run dates, not checkout mtimes or commit dates (backfills).
    dates = {}
    history = root / 'benchmarker' / 'data.js'
    text = history.read_text()
    prefix = 'window.BENCHMARK_DATA = '
    if not text.startswith(prefix):
        raise ValueError('Unexpected benchmark history prefix')
    data = json.loads(text[len(prefix):].strip().removesuffix(';'))
    for runs in data['entries'].values():
        for run in runs:
            sha = run['commit']['id']
            dates[sha] = max(dates.get(sha, 0), run['date'] / 1000)
    cutoff = (time.time() if now is None else now) - max_age_days * 86400
    candidates = []
    for directory in reports.iterdir():
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError(f'Unexpected report entry: {directory}')
        # Legacy reports use short SHAs. Unknown dates expire rather than
        # receiving a new lifetime every time Git checks out the branch.
        date = max((date for sha, date in dates.items()
                    if sha.startswith(directory.name)), default=0)
        size = sum(p.lstat().st_size for p in directory.rglob('*') if p.is_file())
        candidates.append((date, directory, size))
    retained = removed = 0
    for date, directory, size in sorted(candidates, key=lambda item: (item[0], item[1].name), reverse=True):
        if date < cutoff or retained + size > max_bytes:
            shutil.rmtree(directory)
            removed += size
        else:
            retained += size
    print(f'Report retention: {retained} bytes kept; {removed} bytes removed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    prune(parser.parse_args().root)
