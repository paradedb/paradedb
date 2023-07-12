import pytest

from core.sdk.pipeline import Pipeline
from core.extract.postgres import PostgresExtractor
from core.load.elasticsearch import ElasticSearchLoader
from core.transform.custom import CustomEmbedding


def test_postgres_to_elasticsearch(
    postgres_source,
    postgres_transform,
    custom_embedding,
    elasticsearch_sink,
    elasticsearch_target,
    test_index_name,
    test_document_id,
    test_field_name,
    test_vector,
):
    pipeline = Pipeline(
        source=postgres_source,
        transform=postgres_transform,
        embedding=custom_embedding,
        sink=elasticsearch_sink,
        target=elasticsearch_target,
    )

    loader = pipeline._get_loader()
    extractor = pipeline._get_extractor()
    model = pipeline._get_model()

    assert isinstance(loader, ElasticSearchLoader)
    assert isinstance(extractor, PostgresExtractor)
    assert isinstance(model, CustomEmbedding)

    # Run pipe_once()
    pipeline.pipe_once(verbose=True)

    # Check that the embedding was inserted correctly
    response = loader.es.get(index=test_index_name, id=test_document_id)
    assert response["_source"][test_field_name] == test_vector
