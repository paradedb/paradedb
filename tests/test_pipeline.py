from clients.python.retakesearch import Table
from clients.python.retakesearch.search import Search


def test_postgres_to_opensearch(
    postgres_source,
    opensearch_service_and_fastapi_client,
    test_table_name,
    test_primary_key,
    test_column_name,
    test_index_name,
):
    # Initialize a temporary database and associated table
    database = postgres_source
    table = Table(
        name=test_table_name,
        primary_key=test_primary_key,
        columns=[test_column_name],
        neural_columns=[test_column_name],
    )

    # Initialize OpenSearch service & FastAPI Retake Client
    client = opensearch_service_and_fastapi_client

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = client.create_index(test_index_name)
    index.add_source(database, table)

    # Test executing a search query of each type
    bm25_search_query = Search().query("match_all")
    response = index.search(bm25_search_query)
    print(response)

    neural_search_query = Search().neuralQuery("fake data", [test_column_name])
    response = index.search(neural_search_query)
    print(response)
