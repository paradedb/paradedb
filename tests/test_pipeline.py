import pytest

from core.search.pipeline import Pipeline
from core.extract.postgres import PostgresExtractor
from clients.python.retakesearch import Index, Database, Table
from clients.python.retakesearch.client import Client
from clients.python.retakesearch.search import Search


def test_postgres_to_opensearch(
    postgres_source,
    opensearch_client_factory,
    test_table_name,
    test_primary_key,
    test_column_name,
    test_index_name,
):
    ### Integrate with Postgres ###
    # Step 1
    client = opensearch_client_factory

    # Step 2
    database = postgres_source

    # Step 3
    table = Table(
        name=test_table_name,
        primary_key=test_primary_key,
        columns=[test_column_name],
        # Columns passed into neural_columns will have neural search enabled
        # Columns not included will only have keyword search enabled
        neural_columns=[test_column_name],
    )

    # Step 4
    index = client.create_index(test_index_name)

    # Step 5
    index.add_source(database, table)

    ### Execute a Search ###
    # Step 1
    bm25_search = Search().query("match", questions="Who am I?")
    index.search(test_table_name, bm25_search)

    # Step 2
    neural_search = Search().neuralQuery("Who am I?", [test_column_name])
    index.search(test_table_name, neural_search)
