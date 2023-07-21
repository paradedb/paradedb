from unittest.mock import Mock, patch
from core.transform.openai import OpenAIEmbedding


def test_openai_embedding_initialization():
    api_key = "test_api_key"
    model = "test_model"

    openai_embedding = OpenAIEmbedding(api_key, model)

    assert openai_embedding.model == model


def test_openai_embedding_create_embeddings():
    api_key = "test_api_key"
    model = "test_model"
    documents = ["document1", "document2"]
    embeddings = [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]

    with patch("openai.api_key", new_callable=Mock), patch(
        "openai.Embedding.create"
    ) as MockEmbedding:
        MockEmbedding.return_value = embeddings
        openai_embedding = OpenAIEmbedding(api_key, model)
        result = openai_embedding.create_embeddings(documents)

    MockEmbedding.assert_called_once_with(input=[documents], model=model)
    assert result == embeddings
