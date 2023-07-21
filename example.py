from retake import Source, Sink, Transform, Embedding, Target, Pipeline, RealtimeServer
from typing import Union, Dict

source = Source.Postgres(
    user="postgres",
    database="postgres",
    host="postgres-instance-1.chqsp2e4eplp.us-east-2.rds.amazonaws.com",
    password="Password123!",
    port=5432,
)


def transform_func(question: Union[str, None], answer: Union[str, None]) -> str:
    return (question or "") + (answer or "")


def optional_metadata(
    question: Union[str, None], answer: Union[str, None]
) -> Dict[str, str]:
    return {"any_key": "any_value"}


transform = Transform.Postgres(
    # Note: Your table must have a primary key
    primary_key="factor_id",
    relation="ecoinvent_with_types",
    columns=["activity_name", "reference_unit"],
    transform_func=transform_func,
    optional_metadata=optional_metadata,
)

model = Embedding.SentenceTransformer(model="all-MiniLM-L6-v2")

sink = Sink.ElasticSearch(
    host="https://localhost:9200",
    user="elastic",
    password="tQXOkxV-i=e3A6eatRH=",
    ssl_assert_fingerprint="666a46df606dc7626c56bde25c77661819cda1037b06e44f699caec3909fe87c",
)

target = Target.ElasticSearch(index_name="test_index", field_name="test_field")

# sink = Sink.OpenSearch(
#     hosts=[{"host": "localhost", "port": "9200"}],
#     user="admin",
#     password="admin",
#     use_ssl=True,
#     cacerts="/home/mau/Projects/opensearch/root-ca.pem",
# )

# target = Target.OpenSearch(index_name="test_index", field_name="my_field")

pipeline = Pipeline(
    source=source, embedding=model, sink=sink, transform=transform, target=target
)

realtime_server = RealtimeServer(
    host="localhost", broker_port="9094", schema_registry_port="8081"
)

# pipeline.pipe_all(verbose=True)
pipeline.create_real_time(realtime_server)
pipeline.pipe_real_time(realtime_server)
