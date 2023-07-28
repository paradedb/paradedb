import os
import pytest
import psycopg2
import requests

from requests.auth import HTTPBasicAuth

from core.extract.postgres import PostgresSource
from clients.python.retakesearch.client import Client


## Helpers ##


def local_opensearch_client(docker_ip, docker_services):
    print(
        "\nSpinning up local OpenSearch Docker container + fixture (this may take a minute)..."
    )

    def is_responsive(url):
        try:
            response = requests.get(
                url, auth=HTTPBasicAuth("admin", "admin"), verify=False
            )
            return response.status_code == 200
        except Exception as e:
            return False

    port = docker_services.port_for("opensearch", 9200)
    url = f"https://{docker_ip}:{port}"

    print(f"Waiting for OpenSearch instance at {url} to be responsive...")

    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_responsive(url)
    )

    print("OpenSearch instance is responsive!")

    return Client(api_key="retake_test_key", url="http://localhost:9200")


def ci_opensearch_client():
    return Client(api_key="retake_test_key", url="http://localhost:8000")


## Fixtures ##


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
    source = PostgresSource(dsn=dsn)
    return source


# In CI, we run the OpenSearch Docker container separately via a GitHub Action, so we
# create this fixture factory to only launch the container if we're running the test locally.
@pytest.fixture
def opensearch_client_factory(request):
    ci_var = os.getenv("CI")

    if not ci_var:
        print("Testing in local environment...")
        docker_ip = request.getfixturevalue("docker_ip")
        docker_services = request.getfixturevalue("docker_services")
        return local_opensearch_client(docker_ip, docker_services)
    else:
        print("Testing in CI environment...")
        return ci_opensearch_client()
