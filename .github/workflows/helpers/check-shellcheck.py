#!/usr/bin/env python3

import argparse
import glob
import subprocess

DESCRIPTION = """
This script will run shellcheck on all .sh files in the folder the script is called from and
all subfolders.
"""

parser = argparse.ArgumentParser(description=DESCRIPTION)

parser.add_argument(
    "--exclude-dirs",
    help="The list of directories to exclude. Separate using spaces",
    type=str,
    default="",
)

parser.add_argument(
    "--exclude-codes",
    help="The list of codes to exclude for specific files. Separate codes by commas and files:codes pairs by spaces.",
    type=str,
    default="",
)

if __name__ == "__main__":
    args = parser.parse_args()
    exclude_dirs = args.exclude_dirs.split()

    # Obtain list of all .sh files after excluding directories
    sh_files = set(glob.glob("**/*.sh", recursive=True))
    for dir_path in exclude_dirs:
        sh_files -= set(glob.glob(dir_path + "**/*.sh", recursive=True))

    codes_to_exclude = {}
    exclude_codes = args.exclude_codes.split()
    for code in exclude_codes:
        filename, codes = code.split(":")
        # Don't split codes!
        codes_to_exclude[filename] = codes

    for file in sh_files:
        if file not in codes_to_exclude:
            p = subprocess.run(
                "shellcheck {}".format(file), shell=True, capture_output=True
            )
            print(p.stdout.decode())
            if p.returncode != 0:
                print("[Shellcheck did not pass] {}".format(file))
                exit(1)
        else:
            codes = codes_to_exclude[file]
            p = subprocess.run(
                "shellcheck -e {} {}".format(codes, file),
                shell=True,
                capture_output=True,
            )
            print(p.stdout.decode())
            if p.returncode != 0:
                print("[Shellcheck did not pass] {}".format(file))
                exit(1)
        print("[Shellcheck passed] {}".format(file))
