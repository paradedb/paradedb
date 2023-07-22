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

    assert isinstance(pipeline.loader, ElasticSearchLoader)
    assert isinstance(pipeline.extractor, PostgresExtractor)
    assert isinstance(pipeline.model, CustomEmbedding)

    # Run pipe_all()
    pipeline.pipe_all(verbose=True)

    # Check that the embedding was inserted correctly
    response = pipeline.loader.es.get(index=test_index_name, id=test_document_id)
    assert response["_source"][test_field_name] == test_vector
