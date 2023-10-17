/*
 * Copyright 2016 - 2023 Crunchy Data Solutions, Inc.
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

--- System Setup
SET application_name="container_setup";

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE EXTENSION IF NOT EXISTS pgaudit;

ALTER USER postgres PASSWORD 'PG_ROOT_PASSWORD';

CREATE USER "PG_PRIMARY_USER" WITH REPLICATION;
ALTER USER "PG_PRIMARY_USER" PASSWORD 'PG_PRIMARY_PASSWORD';

CREATE USER "PG_USER" LOGIN;
ALTER USER "PG_USER" PASSWORD 'PG_PASSWORD';

CREATE DATABASE "PG_DATABASE";
GRANT ALL PRIVILEGES ON DATABASE "PG_DATABASE" TO "PG_USER";

CREATE TABLE IF NOT EXISTS primarytable (key varchar(20), value varchar(20));
GRANT ALL ON primarytable TO "PG_PRIMARY_USER";

--- PG_DATABASE Setup

\c "PG_DATABASE"

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
CREATE EXTENSION IF NOT EXISTS pgaudit;

--- Verify permissions via PG_USER

\c "PG_DATABASE" "PG_USER";

CREATE SCHEMA IF NOT EXISTS "PG_USER";
