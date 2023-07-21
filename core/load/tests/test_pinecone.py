import pytest
import pinecone
from unittest.mock import Mock, patch, MagicMock
from core.load.pinecone import PineconeLoader
from core.sdk.target import PineconeTarget


@pytest.fixture
def loader():
    return PineconeLoader(api_key="test_key", environment="test_environment")


@pytest.fixture
def target():
    return PineconeTarget(index_name="test_index", namespace="test_namespace")


# Testing _check_index_exists method
@patch("pinecone.describe_index", return_value=Mock())
def test_check_index_exists(describe_index_mock, loader):
    result = loader._check_index_exists("test_index")
    assert result
    describe_index_mock.assert_called_once_with("test_index")


@patch("pinecone.describe_index", side_effect=pinecone.NotFoundException())
def test_check_index_does_not_exists(describe_index_mock, loader):
    result = loader._check_index_exists("test_index")
    assert not result
    describe_index_mock.assert_called_once_with("test_index")


# Testing _get_num_dimensions method
@patch("pinecone.describe_index", return_value=Mock(dimension=5))
def test_get_num_dimensions(describe_index_mock, loader):
    result = loader._get_num_dimensions("test_index")
    assert result == 5
    describe_index_mock.assert_called_once_with("test_index")


# Testing _create_index method
@patch("pinecone.create_index")
def test_create_index(create_index_mock, loader):
    loader._create_index("test_index", 5)
    create_index_mock.assert_called_once_with("test_index", dimension=5)


# Testing check_and_setup_index method
@patch("core.load.pinecone.PineconeLoader._get_num_dimensions", return_value=5)
@patch("core.load.pinecone.PineconeLoader._create_index")
@patch("core.load.pinecone.PineconeLoader._check_index_exists", return_value=False)
def test_check_and_setup_index_not_exists(
    check_index_mock, create_index_mock, get_num_dimensions_mock, loader, target
):
    loader.check_and_setup_index(target, 5)
    check_index_mock.assert_called_once()
    create_index_mock.assert_called_once()
    get_num_dimensions_mock.assert_not_called()


# Testing bulk_upsert_embeddings method
@patch("pinecone.Index", return_value=MagicMock())
def test_bulk_upsert_embeddings(index_mock, loader, target):
    mock_embeddings = [[1, 2, 3], [4, 5, 6]]
    mock_ids = ["test_id1", "test_id2"]
    mock_metadata = [{"key1": "value1"}, {"key2": "value2"}]
    loader.bulk_upsert_embeddings(
        target=target, embeddings=mock_embeddings, ids=mock_ids, metadata=mock_metadata
    )
    index_mock.assert_called_once_with("test_index")
    index_mock.return_value.upsert.assert_called_once_with(
        vectors=[
            {"id": "test_id1", "values": [1, 2, 3], "metadata": {"key1": "value1"}},
            {"id": "test_id2", "values": [4, 5, 6], "metadata": {"key2": "value2"}},
        ],
        namespace="test_namespace",
    )
