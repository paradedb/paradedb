#!/bin/bash

pg_pid=$(pgrep -f "postgres -D /pgdata")
echo pg_pid=${pg_pid}

backrest_env_vars=$( tr '\0' '\n' < /proc/"${pg_pid}"/environ  | grep PGBACKREST_ )
readarray -t backrest_env_var_arr <<< "${backrest_env_vars}"

for env_var in "${!backrest_env_var_arr[@]}"
do
    export "${backrest_env_var_arr[env_var]}"
done
