import pytest
from unittest.mock import Mock, MagicMock, PropertyMock, patch
from qdrant_client import QdrantClient
from qdrant_client.models import (
    Distance,
    VectorParams,
    CollectionsResponse,
    CollectionDescription,
)

from core.load.qdrant import QdrantLoader
from core.sdk.target import QdrantTarget, QdrantSimilarity


class TestQdrantLoader:
    @patch.object(QdrantClient, "__init__", return_value=None)
    def test_constructor_with_url_and_api_key(self, mock_init):
        QdrantLoader(url="test_url", api_key="test_api_key")
        mock_init.assert_called_once_with(url="test_url", api_key="test_api_key")

    @patch.object(QdrantClient, "__init__", return_value=None)
    def test_constructor_with_host_and_port(self, mock_init):
        QdrantLoader(host="localhost", port=8000)
        mock_init.assert_called_once_with(host="localhost", port=8000)

    def test_constructor_with_incorrect_args(self):
        with pytest.raises(ValueError):
            QdrantLoader()

    def test_check_index_exists(self):
        loader = QdrantLoader(host="localhost", port=8000)
        loader.client = Mock()

        mock_response = MagicMock(spec=CollectionsResponse)
        type(mock_response).collections = PropertyMock(
            return_value=[
                CollectionDescription(name="index1"),
                CollectionDescription(name="index2"),
            ]
        )
        loader.client.get_collections = MagicMock(return_value=mock_response)

        assert loader._check_index_exists("index1")
        assert loader._check_index_exists("index2")
        assert not loader._check_index_exists("index3")

    def test_create_index(self):
        loader = QdrantLoader(host="localhost", port=8000)
        loader.client = Mock()
        loader.similarity = QdrantSimilarity.COSINE
        loader._create_index("index1")
        loader.client.recreate_collection.assert_called_once_with(
            collection_name="index1",
            vectors_config=VectorParams(size=100, distance=Distance.COSINE),
        )

    def test_check_and_setup_index(self):
        loader = QdrantLoader(host="localhost", port=8000)
        loader._check_index_exists = MagicMock()
        loader._create_index = MagicMock()
        target = QdrantTarget(index_name="new_index")
        loader.check_and_setup_index(target)
        loader._check_index_exists.assert_called_once_with(index_name="new_index")

    def test_bulk_upsert_embeddings(self):
        loader = QdrantLoader(host="localhost", port=8000)
        loader.client = Mock()
        target = QdrantTarget(index_name="index1")
        embeddings = [[0.1, 0.2], [0.3, 0.4]]
        ids = ["id1", "id2"]
        metadata = [{"key1": "value1"}, {"key2": "value2"}]
        loader.bulk_upsert_embeddings(
            target=target, embeddings=embeddings, ids=ids, metadata=metadata
        )
        loader.client.upsert.assert_called_once()
