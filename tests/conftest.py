import pytest
import psycopg2
import requests
from time import sleep

from requests.auth import HTTPBasicAuth
from requests.exceptions import ConnectionError

from clients.python.retakesearch import Database
from clients.python.retakesearch.client import Client


# Helpers


# Configure pytest-docker to use the root-level docker-compose.yml, to avoid duplication
@pytest.fixture(scope="session")
def docker_compose_file(pytestconfig):
    return pytestconfig.rootpath.joinpath(".", "docker-compose.yml")


def is_opensearch_responsive(url):
    try:
        response = requests.get(url, auth=HTTPBasicAuth("admin", "admin"), verify=False)
        return response.status_code == 200
    except Exception as e:
        return e


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


@pytest.fixture
def postgres_source(
    postgresql, test_table_name, test_primary_key, test_column_name, test_document_id
):
    dsn = f"dbname={postgresql.info.dbname} user={postgresql.info.user} host={postgresql.info.host} port={postgresql.info.port}"

    # Populate DB with test data
    temp_conn = psycopg2.connect(dsn)
    with temp_conn.cursor() as cursor:
        cursor.execute(
            f"CREATE TABLE {test_table_name} ({test_primary_key} varchar PRIMARY KEY, {test_column_name} varchar);"
        )
        cursor.execute(
            f"INSERT INTO {test_table_name} VALUES ('{test_document_id}', 'fake_data1'), ('id2', 'fake_data2'), ('id3', 'fake_data3');"
        )
    temp_conn.commit()
    temp_conn.close()

    # Return Source
    return Database(
        host=postgresql.info.host,
        user=postgresql.info.user,
        password=postgresql.info.password,
        port=postgresql.info.port,
    )


@pytest.fixture(scope="session")
def opensearch_service_and_fastapi_client(docker_ip, docker_services):
    """Ensure that OpenSearch & FastAPI services are up and responsive."""

    print("\nSpinning up OpenSearch service...")
    os_port = docker_services.port_for("core", 9200)
    os_url = f"https://{docker_ip}:{os_port}"

    print(f"Waiting for OpenSearch service at {os_url} to be responsive...")
    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_opensearch_responsive(os_url)
    )
    # We need to wait another 60-90 seconds for it to initialize, after the
    # Docker container is responsive
    sleep(90)
    print("OpenSearch service is responsive!")

    print("\nSpinning up FastAPI service...")
    test_api_key = "retake-test-key"

    fastapi_port = docker_services.port_for("api", 8000)
    fastapi_url = f"http://{docker_ip}:{fastapi_port}"
    ping_url = f"{fastapi_url}/ping"

    print(f"Waiting for FastAPI service at {fastapi_url} to be responsive...")
    docker_services.wait_until_responsive(
        timeout=90.0,
        pause=1,
        check=lambda: is_fastapi_responsive(ping_url, test_api_key),
    )
    print("FastAPI service is responsive!")

    return Client(api_key=test_api_key, url=fastapi_url)
