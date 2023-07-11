import pytest
from unittest.mock import Mock, patch

from core.sdk.pipeline import Pipeline
from core.sdk.source import Source, PostgresSource
from core.sdk.transform import Transform, PostgresTransform
from core.sdk.embedding import (
    Embedding,
    OpenAIEmbedding,
    SentenceTransformerEmbedding,
    CohereEmbedding,
    CustomEmbedding,
)
from core.sdk.sink import Sink, ElasticSearchSink, PineconeSink
from core.sdk.target import Target, ElasticSearchTarget, PineconeTarget

# We assume your_module is the module where the Pipeline and all classes are located.


def test_pipeline_init():
    source = Source.PostgresSource(
        host="test_host",
        user="test_user",
        password="test_password",
        database="test_database",
        port=5432,
    )
    transform = Transform.PostgresTransform(
        relation="test_relation",
        primary_key="test_primary_key",
        columns=["test_column"],
        transform_func=Mock(return_value="transformed_value"),
    )
    embedding = Embedding.OpenAI(api_key="test_api_key", model="test_model")
    sink = Sink.ElasticSearch(cloud_id="test_id")
    target = Target.ElasticSearch(
        index_name="test_index_name", field_name="test_field_name"
    )

    pipeline = Pipeline(source, transform, embedding, sink, target)

    assert isinstance(pipeline.source, PostgresSource)
    assert isinstance(pipeline.transform, PostgresTransform)
    assert isinstance(pipeline.embedding, OpenAIEmbedding)
    assert isinstance(pipeline.sink, ElasticSearchSink)
    assert isinstance(pipeline.target, ElasticSearchTarget)
    # Add more assertions for extractor, loader, model


def test_get_extractor_invalid_source():
    source = "InvalidSource"
    with pytest.raises(ValueError):
        Pipeline._get_extractor(Pipeline, source)


@pytest.mark.parametrize(
    "sink, target, exception_expected",
    [
        (ElasticSearchSink(), "InvalidTarget", True),
        (
            PineconeSink(api_key="test_api_key", environment="test_environment"),
            "InvalidTarget",
            True,
        ),
        (
            "InvalidSink",
            ElasticSearchTarget(
                index_name="test_index_name", field_name="test_field_name"
            ),
            True,
        ),
        (
            "InvalidSink",
            PineconeTarget(index_name="test_index_name", namespace="test_namespace"),
            True,
        ),
        (
            ElasticSearchSink(),
            ElasticSearchTarget(
                index_name="test_index_name", field_name="test_field_name"
            ),
            False,
        ),
        (
            PineconeSink(api_key="test_api_key", environment="test_environment"),
            PineconeTarget(index_name="test_index_name", namespace="test_namespace"),
            False,
        ),
    ],
)
def test_get_loader(sink, target, exception_expected):
    pipeline = Pipeline(
        source=PostgresSource(dsn="test_dsn"),
        transform=PostgresTransform(
            relation="test_relation",
            primary_key="test_primary_key",
            columns=["test_column"],
            transform_func=Mock(return_value="transformed_value"),
        ),
        embedding=OpenAIEmbedding(api_key="test_api_key", model="test_model"),
        sink=sink,
        target=target,
    )
    if exception_expected:
        with pytest.raises(ValueError):
            pipeline._get_loader()
    else:
        # Assert that no exception is raised
        pipeline._get_loader()


@pytest.mark.parametrize(
    "embedding, exception_expected",
    [
        ("InvalidEmbedding", True),
        (OpenAIEmbedding(api_key="test_api_key", model="test_model"), False),
        (SentenceTransformerEmbedding(model="test_model"), False),
        (CohereEmbedding(api_key="test_api_key", model="test_model"), False),
        (CustomEmbedding(func=Mock(return_value="mock_func")), False),
    ],
)
def test_get_model(embedding, exception_expected):
    pipeline = Pipeline(
        source=PostgresSource(dsn="test_dsn"),
        transform=PostgresTransform(
            relation="test_relation",
            primary_key="test_primary_key",
            columns=["test_column"],
            transform_func=Mock(return_value="transformed_value"),
        ),
        embedding=embedding,
        sink=ElasticSearchSink(),
        target=ElasticSearchTarget(
            index_name="test_index_name", field_name="test_field_name"
        ),
    )
    if exception_expected:
        with pytest.raises(ValueError):
            pipeline._get_model()
    else:
        # Assert that no exception is raised
        pipeline._get_model()


# Add more test cases for other methods: _apply_transform, _create_metadata, pipe_once, pipe_real_time, teardown
