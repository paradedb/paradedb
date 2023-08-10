# Flask Demo App

This is a demo Flask app built with `retakesearch`. The app exposes a single `/search` endpoint
that performs a basic neural (i.e. keyword + semantic) search over your Postgres table.

## Get Started

**Prerequisite** You must have a Postgres database with a table that has at least one row and a primary key column.

1. Start the Retake engine. From the `docker` folder of this repo (not the `flask` directory), run

```bash
docker compose up
```

2. `cd` back into this directory

```bash
cd examples/flask
```

3. Create a `.env` file with the following content. Replace all environment variables accordingly.

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

4. Install Poetry (Python dependency manager) and dependencies

```bash
pip install poetry
poetry install
```

5. Run the setup script, which will index the database table specified in the `.env` file:

```bash
python scripts/setup.py
```

6. Start the Flask app

```bash
poetry run flask run
```

That's it! To test the Flask app, try sending it a test POST request

```bash
curl -X POST -H "Content-Type: application/json" -d '{"query": [YOUR QUERY HERE]}' http://127.0.0.1:5000/search
```
