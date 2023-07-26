from retakesearch import Client, Database, Table
from dotenv import load_dotenv
import os
import json

load_dotenv()

client = Client(api_key=os.getenv("RETAKE_API_KEY"), url=os.getenv("RETAKE_API_URL"))

database = Database(
    host=os.getenv("DATABASE_HOST"),
    port=os.getenv("DATABASE_PORT"),
    user=os.getenv("DATABASE_USER"),
    password=os.getenv("DATABASE_PASSWORD"),
)

table = Table(
    name=os.getenv("DATABASE_TABLE_NAME"),
    primary_key=os.getenv("DATABASE_TABLE_PRIMARY_KEY"),
    columns=json.loads(os.getenv("DATABASE_TABLE_COLUMNS")),
    neural_columns=json.loads(os.getenv("DATABASE_TABLE_COLUMNS")),
)

response = client.index(database, table)


def search(query):
    response = client.search(table.name, query)
    return response


print(response)
