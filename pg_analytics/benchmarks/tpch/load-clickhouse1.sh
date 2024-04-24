#!/bin/bash

clickhouse --multiquery < tpch_ch_schema1.sql --time
