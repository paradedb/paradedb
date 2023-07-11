import pytest
import psycopg2
import requests

from psycopg2.extras import LogicalReplicationConnection
from elasticsearch import Elasticsearch
from requests.adapters import HTTPAdapter
from requests.auth import HTTPBasicAuth
from urllib3.util.retry import Retry
from requests.exceptions import ConnectionError

from unittest.mock import Mock, patch

from core.sdk.pipeline import Pipeline
from core.sdk.source import Source, PostgresSource
from core.sdk.transform import Transform, PostgresTransform
from core.sdk.embedding import (
    Embedding,
    OpenAIEmbedding,
    SentenceTransformerEmbedding,
    CohereEmbedding,
    CustomEmbedding,
)
from core.sdk.sink import Sink, ElasticSearchSink, PineconeSink
from core.sdk.target import Target, ElasticSearchTarget, PineconeTarget

# @pytest.fixture
# def postgres_source(postgresql):
#     return Source.Postgres(
#         host=postgresql.info.host,
#         port=postgresql.info.port,
#         user=postgresql.info.user,
#         password=postgresql.info.password,
#         database=postgresql.info.dbname,
#     )

# def test_pipeline(postgres_source):
#     # Set up mock objects
#     transform = Mock(spec=PostgresTransform)
#     embedding = Mock(spec=OpenAIEmbedding)
#     sink = Mock(spec=ElasticSearchSink)
#     target = Mock(spec=ElasticSearchTarget)

#     # Initialize Pipeline with mock objects
#     pipeline = Pipeline(postgres_source, transform, embedding, sink, target)

#     # Test _get_extractor
#     assert isinstance(pipeline._get_extractor(), PostgresExtractor)

#     # Test _get_loader
#     assert isinstance(pipeline._get_loader(), ElasticSearchLoader)

#     # Test _get_model
#     assert isinstance(pipeline._get_model(), OpenAI)

#     # Test _apply_transform and _create_metadata with mock row data
#     row = ("mock_row_data",)
#     assert pipeline._apply_transform(row) == transform.transform_func.return_value
#     if transform.optional_metadata is not None:
#         assert (
#             pipeline._create_metadata(row) == transform.optional_metadata.return_value
#         )
#     else:
#         with pytest.raises(ValueError):
#             pipeline._create_metadata(row)

#     # Mock extractor's extract_all and count methods
#     with patch.object(
#         pipeline.extractor,
#         "extract_all",
#         return_value=[
#             {"rows": ["mock_row_data"], "primary_keys": ["mock_primary_key"]}
#         ],
#     ), patch.object(pipeline.extractor, "count", return_value=100):
#         # Mock loader's check_and_setup_index and bulk_upsert_embeddings methods
#         with patch.object(pipeline.loader, "check_and_setup_index"), patch.object(
#             pipeline.loader, "bulk_upsert_embeddings"
#         ):
#             # Test pipe_once
#             pipeline.pipe_once(verbose=False)
#             pipeline.extractor.count.assert_called_once_with(transform.relation)
#             pipeline.extractor.extract_all.assert_called_once_with(
#                 relation=transform.relation,
#                 columns=transform.columns,
#                 primary_key=transform.primary_key,
#                 chunk_size=100,
#             )
#             pipeline.loader.check_and_setup_index.assert_called_once()
#             pipeline.loader.bulk_upsert_embeddings.assert_called_once()

#     # Test teardown
#     with patch.object(pipeline.extractor, "teardown"):
#         pipeline.teardown()
#         pipeline.extractor.teardown.assert_called_once()

#     # Test pipe_real_time (expected to raise NotImplementedError)
#     with pytest.raises(NotImplementedError):
#         pipeline.pipe_real_time("mock_cdc_server_url", lambda: None, lambda: None)

# from pytest_postgresql import factories

# # Override the fixture with your custom postgresql proc, if needed
# postgresql_proc = factories.postgresql_proc(port=None)
# postgresql = factories.postgresql('postgresql_proc')

@pytest.fixture
def postgres_source(postgresql):
    dsn = f"dbname={postgresql.info.dbname} user={postgresql.info.user} host={postgresql.info.host} port={postgresql.info.port}"
    return PostgresSource(dsn=dsn)

# def test_some_db_interaction(postgres_source):
#     # Use postgres_source to connect to the DB and perform operations
#     transform = Mock(spec=PostgresTransform)
#     embedding = Mock(spec=OpenAIEmbedding)
#     sink = Mock(spec=ElasticSearchSink, cloud_id="test_id")
#     target = Mock(spec=ElasticSearchTarget)

#     # Initialize Pipeline with mock objects
#     pipeline = Pipeline(postgres_source, transform, embedding, sink, target)

#     assert True


def is_responsive(url):
    try:
        print("Sending request", url)
        response = requests.get(url, auth=HTTPBasicAuth('elastic', 'elastic'), verify=False)
        print(response, response.status_code)
        return response.status_code == 200
    except Exception as e:
        print(e)
        return False

@pytest.fixture(scope="session")
def http_service(docker_ip, docker_services):
    """Ensure that HTTP service is up and responsive."""

    # `port_for` takes a container port and returns the corresponding host port
    port = docker_services.port_for("elasticsearch", 9200)
    url = f"https://{docker_ip}:{port}"
    docker_services.wait_until_responsive(
        timeout=90.0, pause=1, check=lambda: is_responsive(url)
    )
    print("Responsive!!")
    return url

def test_status_code(http_service):
    assert True