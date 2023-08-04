from clients.python.retakesearch import Table
from clients.python.retakesearch.search import Search


def test_python_client(retake_client, test_index_name, database):
    table = Table(
        name="city",
        columns=["city_name"],
        transform={"rename": {"city_name": "name"}},
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = retake_client.create_index(test_index_name)
    index.add_source(database, table)

    # Vectorize the field
    index.vectorize(["name"])

    # Test search
    neural_search_query = Search().with_neural("New York City", ["name"])
    response = index.search(neural_search_query)
    assert len(response["hits"]["hits"]) > 0
