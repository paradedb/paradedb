import pytest

# from core.search.pipeline import Pipeline
# from core.extract.postgres import PostgresExtractor


def test_postgres_to_opensearch(
    postgres_source,
    elasticsearch_sink_factory,
    test_index_name,
    test_document_id,
    test_field_name,
    test_vector,
):
    



    pass
    # pipeline = Pipeline()

    # assert isinstance(pipeline.extractor, PostgresExtractor)

    # # Run pipe_all()
    # pipeline.pipe_all(verbose=True)

    # # Check that the embedding was inserted correctly
    # response = pipeline.loader.es.get(index=test_index_name, id=test_document_id)
    # assert response["_source"][test_field_name] == test_vector
