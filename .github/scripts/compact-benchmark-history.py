#!/usr/bin/env python3
"""Compact query history after the gh-pages SQL-reader migration is installed."""
import argparse
import hashlib
import json
import re
from pathlib import Path

PREFIX = 'window.BENCHMARK_DATA = '
REFERENCE = 'sql-extra:sha256:'


def compact(directory):
    data_path = directory / 'data.js'
    extras_path = directory / 'sql-extras.json'
    # The dictionary also marks deployment of chart readers that resolve references.
    if not data_path.exists() or not extras_path.exists():
        print('SQL-reader migration absent; skipping query compaction')
        return
    text = data_path.read_text()
    if not text.startswith(PREFIX):
        raise ValueError('Unexpected benchmark data prefix')
    original = json.loads(text[len(PREFIX):].strip().removesuffix(';'))
    extras = json.loads(extras_path.read_text())
    data = json.loads(json.dumps(original))

    def resolve(extra):
        if REFERENCE not in extra:
            return extra
        prefix, key = extra.rsplit(REFERENCE, 1)
        sql = extras[key]
        if hashlib.sha256(sql.encode()).hexdigest() != key:
            raise ValueError(f'Invalid SQL dictionary hash: {key}')
        return prefix + sql

    for runs in data['entries'].values():
        for run in runs:
            for bench in run['benches']:
                extra = bench.get('extra', '')
                resolved = resolve(extra)
                if REFERENCE in extra or not re.search(r'\b(SELECT|WITH)\b', resolved, re.I):
                    continue
                prefix, separator, sql = resolved.partition('query=')
                if separator:
                    prefix += separator
                else:
                    prefix, sql = '', resolved
                key = hashlib.sha256(sql.encode()).hexdigest()
                extras[key] = sql
                bench['extra'] = prefix + REFERENCE + key

    def expanded(value):
        value = json.loads(json.dumps(value))
        for runs in value['entries'].values():
            for run in runs:
                for bench in run['benches']:
                    if 'extra' in bench:
                        bench['extra'] = resolve(bench['extra'])
        return value

    if expanded(data) != expanded(original):
        raise ValueError('Compaction changed benchmark contents')
    # No semicolon: the benchmark action parses the suffix directly as JSON.
    data_path.write_text(PREFIX + json.dumps(data, separators=(',', ':'), ensure_ascii=False) + '\n')
    extras_path.write_text(json.dumps(extras, separators=(',', ':'), ensure_ascii=False) + '\n')
    print(f'Compacted {data_path}: {len(text.encode())} -> {data_path.stat().st_size} bytes; all results preserved')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    compact(parser.parse_args().directory)
