#!/bin/bash

clickhouse --multiquery < tpch_ch_schema2.sql --time
