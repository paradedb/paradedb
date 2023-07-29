import pytest
import psycopg2
import requests

from requests.auth import HTTPBasicAuth
from requests.exceptions import ConnectionError

from clients.python.retakesearch import Database
from clients.python.retakesearch.client import Client


# Configurations


@pytest.fixture(scope="session")
def docker_compose_file(pytestconfig):
    return pytestconfig.rootpath.joinpath(".", "docker-compose.yml")


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
def test_vector():
    return [0.1, 0.2, 0.3]


@pytest.fixture(scope="session")
def test_index_name():
    return "test_index_name"


@pytest.fixture(scope="session")
def test_field_name():
    return "test_field_name"


@pytest.fixture(scope="session")
def test_document_id():
    return "test_document_id"


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
def opensearch_service(docker_ip, docker_services):
    """Ensure that OpenSearch service is up and responsive."""
    print("\nSpinning up OpenSearch service...")

    def is_responsive(url):
        try:
            response = requests.get(
                url, auth=HTTPBasicAuth("admin", "admin"), verify=False
            )
            return response.status_code == 200
        except Exception as e:
            return e

    port = docker_services.port_for("core", 9200)
    url = f"https://{docker_ip}:{port}"

    print(f"Waiting for OpenSearch service at {url} to be responsive...")

    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_responsive(url)
    )

    print("OpenSearch service is responsive!")
    return url


@pytest.fixture(scope="session")
def fastapi_client(docker_ip, docker_services):
    """Ensure that FastAPI service is up and responsive."""
    print("\nSpinning up FastAPI service...")
    api_key = "retake-test-key"

    def is_responsive(url):
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        }
        try:
            response = requests.get(url, headers=headers, verify=False)
            if response.status_code == 200:
                return True
        except ConnectionError:
            return False

    port = docker_services.port_for("api", 8000)
    url = f"http://{docker_ip}:{port}"
    ping_url = f"{url}/ping"

    print(f"Waiting for FastAPI service at {url} to be responsive...")

    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_responsive(ping_url)
    )

    print("FastAPI service is responsive!")
    return Client(api_key=api_key, url=url)
