#!/bin/bash

clickhouse --multiquery < tpch_ch_schema1.sql --time

clickhouse --multiquery < tpch_ch_schema2.sql --time


