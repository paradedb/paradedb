#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
This script will run shellcheck on all .sh files in the folder the script is
called from and all subfolders.
"""

import argparse
import glob
import subprocess
import sys

parser = argparse.ArgumentParser()

parser.add_argument(
    "--exclude-dirs",
    help="The list of directories to exclude. Separate using spaces",
    type=str,
    default="",
)

parser.add_argument(
    "--exclude-codes",
    help="The list of codes to exclude for specific files. Separate codes by \
          commas and files:codes pairs by spaces.",
    type=str,
    default="",
)

if __name__ == "__main__":
    args = parser.parse_args()
    exclude_dirs = args.exclude_dirs.split()

    # Obtain list of all .sh files after excluding directories
    sh_files = set(glob.glob("**/*.sh", recursive=True))
    for dir_path in exclude_dirs:
        sh_files -= set(glob.glob(f"{dir_path}**/*.sh", recursive=True))

    codes_to_exclude = {}
    exclude_codes = args.exclude_codes.split()
    for code in exclude_codes:
        filename, codes = code.split(":")
        codes_to_exclude[filename] = codes

    for file in sh_files:
        if file not in codes_to_exclude:
            p = subprocess.run(
                f"shellcheck {file}", shell=True, capture_output=True, check=False
            )
            print(p.stdout.decode())
            if p.returncode != 0:
                print(f"[Shellcheck did not pass] {file}")
                sys.exit(1)
        else:
            codes = codes_to_exclude[file]
            p = subprocess.run(
                f"shellcheck -e {codes} {file}",
                shell=True,
                capture_output=True,
                check=False,
            )
            print(p.stdout.decode())
            if p.returncode != 0:
                print(f"[Shellcheck did not pass] {file}")
                sys.exit(1)
        print(f"[Shellcheck passed] {file}")
