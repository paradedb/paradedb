import docker

from clients.python.retakesearch import Table, Database
from clients.python.retakesearch.search import Search


# Because

docker_client = docker.from_env()


def get_matching_containers(partial_name):
    all_containers = docker_client.containers.list(all=True)
    matching_containers = [
        container
        for container in all_containers
        if container.name.endswith(partial_name)
    ]
    return matching_containers


def get_container_ip(container_name, network_name):
    container = docker_client.containers.get(container_name)
    return container.attrs["NetworkSettings"]["Networks"][network_name]["IPAddress"]


def test_postgres_to_opensearch(
    retake_client,
    test_index_name,
):
    # Define in code our PostgreSQL database and associated table as defined in docker-compose.yml

    ## The IP address of host here needs to be that of the postgres container, not that of the localhost, cuz its from
    ## the perspective of within the OpenSearch docker container, which is on a different network!!

    matching_containers = get_matching_containers("-postgres-1")
    pg_container_name = matching_containers[0].name
    print(f"PostgreSQL Docker container name: {pg_container_name}")

    pg_container_network = pg_container_name.split("-")[0] + "_default"
    print(f"PostgreSQL Docker container network: {pg_container_network}")

    pg_container_ip = get_container_ip(pg_container_name, pg_container_network)
    print(f"PostgreSQL Docker container IP: {pg_container_ip}\n")

    database = Database(
        host=pg_container_ip,
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

    index.add_source(database, table)

    # Test that the data was loaded and can be searched
    bm25_search_query = Search().query("match_all")
    response = index.search(bm25_search_query)
    print(response)

    neural_search_query = Search().neuralQuery("fake data", ["city_name"])
    response = index.search(neural_search_query)
    print(response)
