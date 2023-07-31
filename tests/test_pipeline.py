import psycopg2

from clients.python.retakesearch import Table, Database
from clients.python.retakesearch.search import Search


def test_postgres_to_opensearch(
    retake_client,
    test_index_name,
):
    # Define in code our PostgreSQL database and associated table as defined in docker-compose.yml
    database = Database(
        host="127.0.0.1",
        user="postgres",
        password="postgres",
        port="5432",
        dbname="postgres",
    )

    table = Table(
        name="city",
        primary_key="city_id",
        columns=["city_name"],
        neural_columns=["city_name"],
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = retake_client.create_index(test_index_name)

    # TODO:fails here
    index.add_source(database, table)

    # Test that the data was loaded and can be searched
    bm25_search_query = Search().query("match_all")
    response = index.search(bm25_search_query)
    print(response)

    neural_search_query = Search().neuralQuery("fake data", ["city_name"])
    response = index.search(neural_search_query)
    print(response)
