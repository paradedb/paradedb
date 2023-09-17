#!/bin/bash

# TODO: simplify these functions down further (they're still repetitive)
db_query () {
    HOST=$1
    PORT=$2
    DATABASE=$3
    USER=$4
    PASSWORD=$5
    QUERY=$6
    PGPASSWORD=$PASSWORD psql -h $HOST -p $PORT -d $DATABASE -a -U $USER -c "$QUERY"
}

db_file () {
    HOST=$1
    PORT=$2
    DATABASE=$3
    USER=$4
    PASSWORD=$5
    FILE=$6
    PGPASSWORD=$PASSWORD psql -h $HOST -p $PORT -d $DATABASE -a -U $USER -f $FILE
}

load_data () {
    HOST=$1
    PORT=$2
    DATABASE=$3
    USER=$4
    PASSWORD=$5
    WIKI_ARTICLES_FILE=$6

    # First download and unzip the json corpus of wikipedia pages at https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0
    #    and keep it in this folder named as "wiki-articles.json"
    if [ ! -f "$WIKI_ARTICLES_FILE" ]; then
        echo "Please download and unzip the data from https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0 and store it in this folder as wiki-articles.json"
        exit
    fi

    # Create table for json entries and load entries from file into table.
    #     In order to pull entries from your local files, you have to use the combo of cat and COPY FROM STDIN with the -c option
    db_query $HOST $PORT $DATABASE $USER $PASSWORD "DROP TABLE IF EXISTS temp_json;"
    db_query $HOST $PORT $DATABASE $USER $PASSWORD "CREATE TABLE temp_json ( j JSONB );"
    cat $WIKI_ARTICLES_FILE | db_query $HOST $PORT $DATABASE $USER $PASSWORD "COPY temp_json FROM STDIN CSV QUOTE E'\x01' DELIMITER E'\x02';"

    # Load the json data into the wikipedia_articles table
    db_file $HOST $PORT $DATABASE $USER $PASSWORD load_data.sql
}

export -f load_data
export -f db_query