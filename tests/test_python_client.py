from clients.python.retakesearch import Table
from clients.python.retakesearch.search import Search


def test_add_source(client, test_index_name, database):
    table = Table(
        name="city",
        columns=["city_name"],
        transform={"rename": {"city_name": "name"}},
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = client.create_index(test_index_name)
    index.add_source(database, table)

    # Ensure the index exists
    assert test_index_name in client.list_indices()

    # Ensure the index has documents
    assert client.describe_index(test_index_name)["count"] > 0


def test_vectorize(client, test_index_name):
    index = client.get_index(test_index_name)
    index.vectorize(["name"])

    assert (
        client.describe_index(test_index_name)
        .get("vectorized_fields", dict())
        .get("name")
        is not None
    )


def test_search_all(client, test_index_name):
    index = client.get_index(test_index_name)

    query = Search().query("match_all")
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0


def test_semantic_search(client, test_index_name):
    index = client.get_index(test_index_name)

    query = Search().with_semantic("New York City", ["name"])
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0


def test_neural_search(client, test_index_name):
    index = client.get_index(test_index_name)

    query = Search().with_neural("New York City", ["name"])
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0
