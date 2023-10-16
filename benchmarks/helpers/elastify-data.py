# -*- coding: utf-8 -*-
import os
import sys

if len(sys.argv[1:]) < 3:
    print(
        "Correct usage: elastify-data.py <wiki_articles_file> \
         <bulk_output_folder> <desired_size>"
    )
    sys.exit(1)

wiki_articles_filename = sys.argv[1]
bulk_output_foldername = sys.argv[2]
desired_size = int(sys.argv[3])

if not os.path.exists(bulk_output_foldername):
    os.makedirs(bulk_output_foldername)

elastic_create_entry = '{"index":{}}\n'

with open(wiki_articles_filename, "r") as wiki_articles_file:
    max_file_entries = 5000
    total_num_written = 0
    bo_num = 0
    bulk_output_file = None

    for line in wiki_articles_file:
        if total_num_written == desired_size:
            break
        if total_num_written % max_file_entries == 0:
            if bulk_output_file:
                bulk_output_file.close()
            bo_num += 1
            bulk_output_filename = os.path.join(
                bulk_output_foldername, f"{desired_size}_{bo_num}.json"
            )
            bulk_output_file = open(bulk_output_filename, "w")

        bulk_output_file.write(elastic_create_entry)
        bulk_output_file.write(line)
        total_num_written += 1

    if bulk_output_file:
        bulk_output_file.write("\n")
        bulk_output_file.close()
