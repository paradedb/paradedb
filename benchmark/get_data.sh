#!/bin/bash

# STEPS:
# 1. Download data
# 2. Load data into database
# 3. Run and time search queries on database
# 3a. Randomize search terms to be tested
# 4. Destroy data table

# First download and unzip the json corpus of wikipedia pages at https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0 
#    and keep it in this folder named as "wiki-articles.json"
WIKI_ARTICLES_FILE=wiki-articles.json
if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
	echo "Please download and unzip the data from https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0 and store it in this folder as wiki-articles.json"
	exit
fi

# Next, run `\i <path_to>/load_data.sql` to load all articles into database