# -*- coding: utf-8 -*-
"""
Module to convert the Wiki articles dataset to a format suitable for
Elasticsearch bulk insertion.
"""

import os
import sys

if len(sys.argv[1:]) < 3:
    print(
        "Correct usage: elastify-data.py <wiki_articles_file> \
         <bulk_output_folder> <desired_size>"
    )
    sys.exit(1)

WIKI_ARTICLES_FILENAME = sys.argv[1]
BULK_OUTPUT_FOLDERNAME = sys.argv[2]
DESIRED_SIZE = int(sys.argv[3])

if not os.path.exists(BULK_OUTPUT_FOLDERNAME):
    os.makedirs(BULK_OUTPUT_FOLDERNAME)

ELASTIC_CREATE_ENTRY = '{"index":{}}\n'

with open(WIKI_ARTICLES_FILENAME, "r", encoding="utf-8") as wiki_articles_file:
    MAX_FILE_ENTRIES = 5000
    total_num_written = 0
    bo_num = 0
    bulk_output_file = None

    for line in wiki_articles_file:
        if total_num_written == DESIRED_SIZE:
            break
        if total_num_written % MAX_FILE_ENTRIES == 0:
            if bulk_output_file:
                bulk_output_file.close()
            bo_num += 1
            bulk_output_filename = os.path.join(
                BULK_OUTPUT_FOLDERNAME, f"{DESIRED_SIZE}_{bo_num}.json"
            )
            with open(bulk_output_filename, "w", encoding="utf-8") as bulk_output_file:
                bulk_output_file.write(ELASTIC_CREATE_ENTRY)
                bulk_output_file.write(line)
                total_num_written += 1
        else:
            bulk_output_file.write(ELASTIC_CREATE_ENTRY)
            bulk_output_file.write(line)
            total_num_written += 1
