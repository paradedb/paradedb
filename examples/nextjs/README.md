This is a demo Next.js app built with `retake-search`. The app enables users to perform search over your Postgres table.

## Get Started

**Prerequisite** You must have a Postgres database with a table that has at least one row and a primary key column.

1. Start the Retake engine. From the `docker` folder of this repo (not the `flask` directory), run

```bash
docker compose up
```

2. `cd` back into this directory

```bash
cd examples/nextjs
```

3. In the root `nextjs` directory, create a `.env` file with the following content. Replace all environment
   variables accordingly.

```bash
# Host URL of your Postgres database
DATABASE_HOST=***
# Username of your Postgres database
DATABASE_USER=***
# Password of your Postgres database
DATABASE_PASSWORD=***
# Port number of your Postgres database
DATABASE_PORT=5432
# Database name
DATABASE_NAME=***

# Table name that you wish to index/search
DATABASE_TABLE_NAME=my_table_name
# Primary key column name of the above table
DATABASE_TABLE_PRIMARY_KEY=table_pk
# Array of columns that you wish to search over
DATABASE_TABLE_COLUMNS=["column_1", "column_2"]

# See docker-compose.yml in project root directory for
# default values
RETAKE_API_KEY=retake-test-key
RETAKE_API_URL=http://localhost:8000
```

4. Run the setup script, which will index the database table specified in the `.env` file:

```bash
npm run setup
```

5. Start the Next.js app

```bash
npm start
```
