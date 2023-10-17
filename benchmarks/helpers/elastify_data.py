# -*- coding: utf-8 -*-

"""
Module to convert the Wiki articles dataset to a format suitable for
Elasticsearch bulk insertion.
"""

import os
import sys

ELASTIC_CREATE_ENTRY = '{"index":{}}\n'
MAX_FILE_ENTRIES = 5000


def write_to_bulk_file(filename, lines):
    """
    Writes the given lines to the given file, in bulk.
    """
    with open(filename, "w", encoding="utf-8") as bulk_output_file:
        for line in lines:
            bulk_output_file.write(ELASTIC_CREATE_ENTRY)
            bulk_output_file.write(line)


def elastify_data(wiki_articles_filename, bulk_output_foldername, desired_size):
    """
    Converts Wiki articles dataset to a format suitable for Elasticsearch bulk
    insertion.

    Parameters:
    - wiki_articles_filename (str): Path to the input file with Wiki articles.
    - bulk_output_foldername (str): Path to the output directory.
    - desired_size (int): Desired number of articles to process.
    """
    if not os.path.exists(bulk_output_foldername):
        os.makedirs(bulk_output_foldername)

    total_num_written = 0
    bo_num = 1
    bulk_output_filename = os.path.join(
        bulk_output_foldername, f"{desired_size}_{bo_num}.json"
    )

    lines_to_write = []

    with open(wiki_articles_filename, "r", encoding="utf-8") as wiki_articles_file:
        for line in wiki_articles_file:
            if total_num_written == desired_size:
                write_to_bulk_file(bulk_output_filename, lines_to_write)
                break

            if total_num_written > 0 and total_num_written % MAX_FILE_ENTRIES == 0:
                write_to_bulk_file(bulk_output_filename, lines_to_write)
                lines_to_write = []  # Reset the buffer
                bo_num += 1
                bulk_output_filename = os.path.join(
                    bulk_output_foldername, f"{desired_size}_{bo_num}.json"
                )

            lines_to_write.append(line)
            total_num_written += 1

        if lines_to_write:
            write_to_bulk_file(bulk_output_filename, lines_to_write)


if __name__ == "__main__":
    if len(sys.argv[1:]) < 3:
        print(
            "Correct usage: elastify_data.py <wiki_articles_file>"
            " <bulk_output_folder> <desired_size>"
        )
        sys.exit(1)

    WIKI_ARTICLES_FILENAME = sys.argv[1]
    BULK_OUTPUT_FOLDERNAME = sys.argv[2]
    DESIRED_SIZE = int(sys.argv[3])

    elastify_data(WIKI_ARTICLES_FILENAME, BULK_OUTPUT_FOLDERNAME, DESIRED_SIZE)
