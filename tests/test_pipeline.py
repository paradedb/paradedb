from clients.python.retakesearch import Table
from clients.python.retakesearch.search import Search


def test_postgres_to_opensearch(
    opensearch_service,
    postgres_source,
    fastapi_client,
    test_table_name,
    test_primary_key,
    test_column_name,
    test_index_name,
):
    # Initialize Retake Client (OpenSearch also needs to be running)
    client = fastapi_client

    # Initialize a temporary database and associated table
    database = postgres_source
    table = Table(
        name=test_table_name,
        primary_key=test_primary_key,
        columns=[test_column_name],
        neural_columns=[test_column_name],
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = client.create_index(test_index_name)
    index.add_source(database, table)

    # Test executing a search query of each type
    bm25_search_query = Search().query("match", test_column_name="Who am I?")
    index.search(bm25_search_query)

    neural_search_query = Search().neuralQuery("Who am I?", [test_column_name])
    index.search(neural_search_query)
