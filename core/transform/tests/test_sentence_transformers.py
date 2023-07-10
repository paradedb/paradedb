import pytest
from unittest.mock import Mock, MagicMock, patch
from typing import List
from core.transform.base import Embedding
from core.transform.sentence_transformers import SentenceTransformerEmbedding

from unittest.mock import PropertyMock


def test_sentence_transformer_embedding_initialization():
    model = "all-MiniLM-L6-v2"

    sentence_transformers_embedding = SentenceTransformerEmbedding(model)

    assert sentence_transformers_embedding.model is not None


def test_sentence_transformer_create_embeddings():
    model_name = "all-MiniLM-L6-v2"
    documents = ["doc"]

    sentence_transformer_embedding = SentenceTransformerEmbedding(model_name)
    result = sentence_transformer_embedding.create_embeddings(documents)

    # Having trouble with the mock_instance.encode call,
    # so using the real embeddings output for now
    assert len(result[0]) == 384
    assert result[0][0] == -0.06271601468324661
