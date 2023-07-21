from unittest.mock import Mock
from typing import List, Callable
from core.transform.custom import CustomEmbedding


def test_custom_embedding_create_embeddings():
    documents = ["doc1", "doc2", "doc3"]
    embeddings = [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9]]

    mock_func: Callable[[List[str]], List[List[float]]] = Mock()
    mock_func.return_value = embeddings

    custom_embedding = CustomEmbedding(mock_func)

    result = custom_embedding.create_embeddings(documents)

    mock_func.assert_called_once_with(documents)
    assert result == embeddings
