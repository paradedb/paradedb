import pytest
import numpy as np
from unittest.mock import patch, Mock, MagicMock
from typing import List
import importlib


@patch("sentence_transformers.SentenceTransformer", autospec=True)
def test_sentence_transformer_embedding_initialization(mocked_transformer):
    model = "all-MiniLM-L6-v2"
    SentenceTransformerEmbedding = importlib.import_module(
        "core.transform.sentence_transformers"
    ).SentenceTransformerEmbedding
    sentence_transformers_embedding = SentenceTransformerEmbedding(model)
    assert sentence_transformers_embedding.model is not None


@patch.dict("sys.modules", {"core.transform.sentence_transformers": MagicMock()})
def test_sentence_transformer_create_embeddings():
    from core.transform.sentence_transformers import SentenceTransformerEmbedding

    model_name = "all-MiniLM-L6-v2"
    documents = ["doc"]
    mock_embeddings = np.array([[0.1, 0.2, 0.3]])

    mock_instance = MagicMock()
    mock_instance.create_embeddings.return_value = mock_embeddings.tolist()

    SentenceTransformerEmbedding.return_value = mock_instance

    embedding = SentenceTransformerEmbedding(model_name)
    result = embedding.create_embeddings(documents)

    mock_instance.create_embeddings.assert_called_once_with(documents)
    assert np.array_equal(result, mock_embeddings.tolist())
