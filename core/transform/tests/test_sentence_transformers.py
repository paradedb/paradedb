import numpy as np

from unittest.mock import patch
from core.transform.sentence_transformers import SentenceTransformerEmbedding


def test_sentence_transformer_embedding_initialization():
    model = "all-MiniLM-L6-v2"

    with patch("core.transform.sentence_transformers.SentenceTransformer"):
        sentence_transformers_embedding = SentenceTransformerEmbedding(model)

        assert sentence_transformers_embedding.model is not None


def test_sentence_transformer_create_embeddings():
    model_name = "all-MiniLM-L6-v2"
    documents = ["doc"]
    mock_embeddings = np.array([[0.1, 0.2, 0.3]])

    with patch(
        "core.transform.sentence_transformers.SentenceTransformer"
    ) as MockedTransformer:
        mock_instance = MockedTransformer.return_value
        mock_instance.encode.return_value = mock_embeddings

        sentence_transformer_embedding = SentenceTransformerEmbedding(model_name)
        result = sentence_transformer_embedding.create_embeddings(documents)

        mock_instance.encode.assert_called_once_with(documents)
        assert result == mock_embeddings.tolist()
