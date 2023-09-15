import os
import sys

if len(sys.argv[1:]) < 2:
	print("Correct usage: elastify-data.py <wiki_articles_file> <bulk_output_file>")

wiki_articles_filename = sys.argv[1]
bulk_output_foldername = sys.argv[2]

if not os.path.exists(bulk_output_foldername):
	os.makedirs(bulk_output_foldername)

elastic_create_entry = '{"index":{}}\n'

wiki_articles_file = open(wiki_articles_filename, 'r')

max_file_entries = 5000
total_num_written = 0
bo_num = 0

bulk_output_file = None

for line in wiki_articles_file.readlines():
	if total_num_written % max_file_entries == 0:
		if bulk_output_file is not None:
			bulk_output_file.close()
			print("Wrote file {}".format(bo_num))
		bo_num += 1
		bulk_output_file = open(os.path.join(bulk_output_foldername, "{}.json".format(bo_num)), 'w')
	bulk_output_file.write(elastic_create_entry)
	bulk_output_file.write(line)
	total_num_written += 1

bulk_output_file.write('\n')