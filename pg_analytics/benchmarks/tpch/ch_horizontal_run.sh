#!/usr/bin/env bash

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0.  If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.
#
# Copyright 2017-2022 MonetDB Solutions B.V.

usage() {
    echo "Usage: $0 --db <db> [--number <repeats>] [--db <d>] [--output <file>]"
    echo "Run the TPC-H queries a number of times and report timings."
    echo ""
    echo "Options:"
    echo "  -d, --db <db>                     The database"
    echo "  -n, --number <repeats>            How many times to run the queries. Default=1"
    echo "  -o, --output <file>               Where to append the output. Default=timings.csv"
    echo "  -v, --verbose                     More output"
    echo "  -h, --help                        This message"
}

dbname="SF-0_01"
nruns=1


while [ "$#" -gt 0 ]
do
    case "$1" in
        -d|--db)
            dbname=$2
            shift
            shift
            ;;
        -n|--number)
            nruns=$2
            shift
            shift
            ;;
        -o|--output)
            nruns=$2
            shift
            shift
            ;;            
        -v|--verbose)
            set -x
            set -v
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "$0: unknown argument $1"
            usage
            exit 1
            ;;
    esac
done

if [ -z "$dbname" ]; then
    usage
    exit 1
fi

output="$dbname.timings.csv"

echo "# Database,Query,Min,Max,Average,Error" | tee -a "$output"


for q in *.sql
do

    max=0
    min=9999999
    sum=0

    for j in $(seq 1 $nruns)
    do
        s=$(date +%s.%N)
	timeout 3600s clickhouse client -n --queries-file $q -d $dbname
	err=$?

	x=$(date +%s.%N)
	elapsed=$(echo "scale=4; $x - $s" | bc)	

	# calculate max, min, avg
	# using bc cmps to have floating point precission
	if [ $(echo "$elapsed > $max" | bc) -eq 1 ] 
	then
	    max=$elapsed
	fi

	if [ $(echo "$elapsed < $min" | bc) -eq 1 ]
        then
            min=$elapsed
        fi

	sum=$(echo "$elapsed + $sum" | bc)

    done

    avg=$(echo "scale=4; $sum/$nruns" | bc)

    echo "$dbname,"$(basename $q .sql)",$min,$max,$avg,$err" | tee -a "$output"

done


