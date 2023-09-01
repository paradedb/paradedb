#!/bin/bash

# First download and unzip the json corpus of wikipedia pages at https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0 
#    and keep it in this folder named as "wiki-articles.json"

# Test with the 1000 file smaller set first
WIKI_ARTICLES_FILE=wiki-articles-1000.json
if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
	echo "Please download and unzip the data from https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0 and store it in this folder as wiki-articles.json"
	exit
fi

# Create database

# Load wikipedia articles into a postgres benchmark database
# TODO: figure out how to keep the escaped characters in the loaded json because it's NOT working
###
BEGIN;

CREATE TEMPORARY TABLE temp_json (values TEXT) ON COMMIT DROP;
COPY temp_json FROM '/Users/suriya-retake/Documents/paradedb/benchmark/wiki-articles-1000.json';

-- INSERT INTO wiki_articles ("url", "title", "body")

SELECT values->>'url' AS url,
       values->>'title' AS title,
       values->>'body' AS body
FROM   (
       SELECT json_array_elements(replace(values,'\','\\')::json) AS values
       FROM temp_json
	   ) A;

COMMIT;
###