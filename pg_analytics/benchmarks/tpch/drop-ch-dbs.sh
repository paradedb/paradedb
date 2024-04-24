#!/bin/bash

# Get list of databases
databases=$(clickhouse-client -q "SHOW DATABASES" | tail -n +2)

# Iterate over each database and drop all tables
for database in $databases; do
    tables=$(clickhouse-client -q "SHOW TABLES FROM $database" | tail -n +2)
    for table in $tables; do
        clickhouse-client -q "DROP TABLE IF EXISTS $database.$table"
        echo "Dropped table $database.$table"
    done
done

echo "All tables dropped"
