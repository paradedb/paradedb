from time import sleep

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


def test_vectorize(client, test_index_name):
    index = client.get_index(test_index_name)
    index.vectorize(["name"])

    assert (
        client.describe_index(test_index_name)
        .get("vectorized_fields", dict())
        .get("name")
        .get("type")
        == "knn_vector"
    )


def test_describe_index(client, test_index_name):
    assert client.describe_index(test_index_name)["count"] > 0


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


def test_upsert(client, test_index_name):
    index = client.get_index(test_index_name)

    index.upsert(
        documents=[
            {
                "name": "Retakeville",
            },
            {"name": "Vectorvilla"},
        ],
        ids=["retakeville-id", "vectorvilla-id"],
    )

    # Temporary: Allow time for upsert to complete
    sleep(2)

    query = Search().query("match", name="Retakeville")
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0

    query = Search().query("match", name="Vectorvilla")
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0


def test_create_field(client, test_index_name):
    index = client.get_index(test_index_name)

    index.create_field("population", "float")

    assert (
        client.describe_index(test_index_name)
        .get("fields")
        .get("population")
        .get("type")
        == "float"
    )


def test_delete_index(client, test_index_name):
    client.delete_index(test_index_name)
    assert test_index_name not in client.list_indices()
