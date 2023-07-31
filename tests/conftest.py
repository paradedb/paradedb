import pytest
import psycopg2
import requests

from requests.auth import HTTPBasicAuth
from requests.exceptions import ConnectionError

from clients.python.retakesearch.client import Client


# Helpers


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


def is_postgres_responsive(url, port):
    try:
        conn = psycopg2.connect(
            dbname="postgres", user="postgres", password="postgres", host=url, port=port
        )
        conn.close()
        return True
    except psycopg2.OperationalError:
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
def retake_client(docker_ip, docker_services):
    """Ensure that PostgreSQL, OpenSearch & FastAPI services are up and responsive."""

    print("\nSpinning up PostgreSQL service...")
    pg_port = docker_services.port_for("postgres", 5432)
    pg_url = docker_ip

    print(f"Waiting for PostgreSQL service at {pg_url}:{pg_port} to be responsive...")
    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_postgres_responsive(pg_url, pg_port)
    )

    print("PostgreSQL service is responsive!\n\nSpinning up OpenSearch service...")

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
