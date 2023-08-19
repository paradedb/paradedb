import math
import random
import pytest

from time import sleep
from faker import Faker

from core.search.index import default_engine
from clients.python.retakesearch import Table, Search, Q

fake = Faker()


def generate_random_vector(dim=20, min_val=-1, max_val=1):
    vector = [random.uniform(min_val, max_val) for _ in range(dim)]
    magnitude = math.sqrt(sum([x**2 for x in vector]))
    return [x / magnitude for x in vector]


def test_add_source(client, test_index_name, database):
    table = Table(
        name="city",
        columns=["city_name", "country_name"],
        transform={"rename": {"city_name": "name"}},
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = client.create_index(test_index_name)
    index.add_source(database, table)

    # Ensure the index exists
    assert test_index_name in client.list_indices()


def test_null_values_handled(client, test_index_name):
    # Sometimes add_source takes a few seconds
    sleep(10)

    index = client.get_index(test_index_name)

    query = Search().query("match", name="Retake City")
    response = index.search(query)

    assert "name" in response["hits"]["hits"][0]["_source"]
    assert "country_name" not in response["hits"]["hits"][0]["_source"]

    query = Search().query("match", country_name="Retake Country")
    response = index.search(query)

    assert "name" not in response["hits"]["hits"][0]["_source"]
    assert "country_name" in response["hits"]["hits"][0]["_source"]


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

    assert (
        client.describe_index(test_index_name)
        .get("vectorized_fields", dict())
        .get("name")
        .get("method")
        .get("engine")
        == default_engine
    )

    index.vectorize(["country_name"], engine="faiss", space_type="l2")

    assert (
        client.describe_index(test_index_name)
        .get("vectorized_fields", dict())
        .get("country_name")
        .get("method")
        .get("engine")
        == "faiss"
    )

    assert (
        client.describe_index(test_index_name)
        .get("vectorized_fields", dict())
        .get("country_name")
        .get("method")
        .get("space_type")
        == "l2"
    )


def test_vectorize_with_other_models(client):
    index = client.create_index("temp_index")

    index.vectorize(
        ["field1", "field2"],
        space_type="l2",
        engine="faiss",
        model_name="huggingface/sentence-transformers/paraphrase-MiniLM-L3-v2",
        model_dimension=768,
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

    query = Search().with_semantic("US", ["country_name"])
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0


def test_neural_search(client, test_index_name):
    index = client.get_index(test_index_name)

    query = Search().with_neural("New York City", ["name"])
    response = index.search(query)
    assert len(response["hits"]["hits"]) > 0

    query = Search().with_neural("US", ["country_name"])
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


def test_create_vector_field(client, test_index_name):
    index = client.get_index(test_index_name)

    # Expect Exception if knn_vector field is created without proper arguments
    with pytest.raises(Exception):
        index.create_field("my_vector", "knn_vector")

    # Expect Exception if space_type and engine are not compatible
    with pytest.raises(Exception):
        index.create_field(
            "my_vector",
            "knn_vector",
            dimension=20,
            space_type="cosinesimil",
            engine="faiss",
        )

    # Create fields
    index.create_field(
        "faiss_vector",
        "knn_vector",
        dimension=20,
        space_type="l2",
        engine="faiss",
    )

    index.create_field(
        "lucene_vector",
        "knn_vector",
        dimension=20,
        space_type="cosinesimil",
        engine="lucene",
    )


def test_upsert_vectors(client, test_index_name):
    index = client.get_index(test_index_name)

    documents = [
        {
            "faiss_vector": generate_random_vector(),
            "lucene_vector": generate_random_vector(),
            "city": fake.city(),
        }
        for _ in range(50)
    ]

    index.upsert(documents=documents, ids=list(range(50)))
    assert client.describe_index(test_index_name).get("count") >= 50


def test_vector_search(client, test_index_name):
    index = client.get_index(test_index_name)

    # It takes some time to index
    sleep(5)

    # Test search over lucene field
    query = Search().with_nearest_neighbor(
        vector=generate_random_vector(), field="lucene_vector", k=10
    )[0:10]

    response = index.search(query)
    assert len(response["hits"]["hits"]) == 10

    # Test search over faiss field
    query = Search().with_nearest_neighbor(
        vector=generate_random_vector(), field="faiss_vector", k=20
    )[0:20]

    response = index.search(query)
    assert len(response["hits"]["hits"]) == 20


def test_vector_search_with_filters(client, test_index_name):
    index = client.get_index(test_index_name)

    query = Search().with_nearest_neighbor(
        vector=generate_random_vector(),
        field="lucene_vector",
        k=25,
        filter=Q("match", city="New"),
    )

    response = index.search(query)
    cities = [hit["_source"]["city"] for hit in response["hits"]["hits"]]
    assert all(["New" in city for city in cities])


def test_delete_index(client, test_index_name):
    client.delete_index(test_index_name)
    assert test_index_name not in client.list_indices()
