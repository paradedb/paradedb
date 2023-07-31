import docker

from clients.python.retakesearch import Table, Database
from clients.python.retakesearch.search import Search


# Load the Docker client created by pytest-docker
docker_client = docker.from_env()


# Get the container name of a running container by partial name, since pytest-docker
# generates containers of format pytestXYZ123-servicename-1 (i.e. pytest123456-postgres-1)
def get_matching_containers(partial_name):
    all_containers = docker_client.containers.list(all=True)
    matching_containers = [
        container
        for container in all_containers
        if container.name.endswith(partial_name)
    ]
    return matching_containers


# Retrieve the IP address of a container on a given network, since we need the internal IP address of the
# container, instead of the localhost IP address, since the OpenSearch and the Postgres containers need to
# communicate with each other within the same Docker network
def get_container_ip(container_name, network_name):
    container = docker_client.containers.get(container_name)
    return container.attrs["NetworkSettings"]["Networks"][network_name]["IPAddress"]


# Tests


def test_postgres_to_opensearch(
    retake_client,
    test_index_name,
):
    # Get the container name, network, and IP address of the PostgreSQL Docker container spun up by pytest-docker
    # as part of the retake_client fixture
    matching_containers = get_matching_containers("-postgres-1")
    pg_container_name = matching_containers[0].name
    print(f"PostgreSQL Docker container name: {pg_container_name}")

    pg_container_network = pg_container_name.split("-")[0] + "_default"
    print(f"PostgreSQL Docker container network: {pg_container_network}")

    pg_container_ip = get_container_ip(pg_container_name, pg_container_network)
    print(f"PostgreSQL Docker container IP: {pg_container_ip}\n")

    # Create adatabase and a table object for our PostgreSQL container
    database = Database(
        host=pg_container_ip,
        user="postgres",
        password="postgres",
        port="5432",
        dbname="postgres",
    )

    table = Table(name="city", primary_key="city_id", columns=["city_name"])

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = retake_client.create_index(test_index_name)
    index.add_source(database, table)

    neural_search_query = Search().neuralQuery("New York City", ["city_name"])
    response = index.search(neural_search_query)
    assert len(response["hits"]["hits"]) > 0
