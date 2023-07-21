from unittest.mock import Mock, patch
from core.transform.cohere import CohereEmbedding


def test_cohere_embedding_initialization():
    api_key = "test_api_key"
    model = "test_model"

    # Mock cohere.Client object
    with patch("cohere.Client") as MockClient:
        cohere_embedding = CohereEmbedding(api_key, model)

    MockClient.assert_called_once_with(api_key)
    assert cohere_embedding.model == model


def test_cohere_embedding_create_embeddings():
    api_key = "test_api_key"
    model = "test_model"
    documents = ["document1", "document2"]
    embeddings = [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]

    # Mock cohere.Client object and its embed method
    with patch("cohere.Client") as MockClient:
        mock_embed_result = Mock()
        mock_embed_result.embeddings = embeddings
        MockClient.return_value.embed.return_value = mock_embed_result

        cohere_embedding = CohereEmbedding(api_key, model)
        result = cohere_embedding.create_embeddings(documents)

    MockClient.return_value.embed.assert_called_once_with(texts=documents, model=model)
    assert result == embeddings
