import docker
import pytest
import requests

from requests.auth import HTTPBasicAuth
from requests.exceptions import ConnectionError

from clients.python.retakesearch import Client, Database

# Helpers

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


def is_opensearch_responsive(url):
    try:
        response = requests.get(url, auth=HTTPBasicAuth("admin", "admin"), verify=False)
        if response.status_code == 200:
            health = response.json()
            return health["status"] in ["green", "yellow"]

        return False
    except Exception:
        return False


def is_fastapi_responsive(url, test_api_key):
    headers = {
        "Authorization": f"Bearer {test_api_key}",
    }
    try:
        response = requests.get(url, headers=headers, verify=False)
        return response.status_code == 200
    except ConnectionError:
        return False


# Fixtures


@pytest.fixture(scope="session")
def test_table_name():
    return "test_table"


@pytest.fixture(scope="session")
def test_column_name():
    return "test_column"


@pytest.fixture(scope="session")
def test_primary_key():
    return "test_pk"


@pytest.fixture(scope="session")
def test_index_name():
    return "test_index_name"


@pytest.fixture(scope="session")
def test_document_id():
    return "test_document_id"


@pytest.fixture(scope="session")
def client(docker_ip, docker_services):
    """Ensure that PostgreSQL, OpenSearch & FastAPI services are up and responsive."""
    print("\nSpinning up OpenSearch service...")
    os_port = docker_services.port_for("core", 9200)
    os_url = f"https://{docker_ip}:{os_port}/_cluster/health"

    print(f"Waiting for OpenSearch service at {os_url} to be responsive...")
    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_opensearch_responsive(os_url)
    )

    print("OpenSearch service is responsive!\n\nSpinning up FastAPI service...")

    test_api_key = "retake-test-key"
    fastapi_port = docker_services.port_for("api", 8000)
    fastapi_url = f"http://{docker_ip}:{fastapi_port}"
    ping_url = f"{fastapi_url}"

    print(f"Waiting for FastAPI service at {fastapi_url} to be responsive...")
    docker_services.wait_until_responsive(
        timeout=90.0,
        pause=1,
        check=lambda: is_fastapi_responsive(ping_url, test_api_key),
    )
    print("FastAPI service is responsive!\n")

    return Client(api_key=test_api_key, url=fastapi_url)


@pytest.fixture(scope="session")
def database():
    # Get the container name, network, and IP address of the PostgreSQL Docker container spun up by pytest-docker
    # as part of the retake_client fixture
    matching_containers = get_matching_containers("-postgres-1")
    pg_container_name = matching_containers[0].name
    print(f"PostgreSQL Docker container name: {pg_container_name}")

    pg_container_network = pg_container_name.split("-")[0] + "_default"
    print(f"PostgreSQL Docker container network: {pg_container_network}")

    pg_container_ip = get_container_ip(pg_container_name, pg_container_network)
    print(f"PostgreSQL Docker container IP: {pg_container_ip}\n")

    return Database(
        host=pg_container_ip,
        user="postgres",
        password="postgres",
        port=5432,
        dbname="postgres",
    )
