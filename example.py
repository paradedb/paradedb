from retake import Source, Transform, Embedding, Sink, Target, Pipeline


def optional_transform(column1: str, column2: str) -> str:
    return (column1 or "") + (column2 or "") + "any other transform"


def optional_metadata(column1: str, column2: str):
    return {"some_metadata": (column1 or "") + (column2 or "") + "any other metadata"}


# Define Source
my_source = Source.Postgres(
    user="postgres",
    password="postgres",
    database="postgres",
    host="localhost",
    port=5432,
)

# Define Transformations
my_transform = Transform.Postgres(
    relation="customers",
    primary_key="id",
    columns=["id", "name"],
    transform_func=optional_transform,
    optional_metadata=optional_metadata,
)

# Define Embedding Model
my_model = Embedding.SentenceTransformer(model="all-MiniLM-L6-v2")

my_sink = Sink.ElasticSearch(
    host="https://localhost:9200",
    user="elastic",
    password="elastic",
    # ssl_assert_fingerprint="27f9642e482a78c2b1393b0c44d9e748f94e1bee2e1e41bda56d8df5b5cab47a"
)

my_target = Target.ElasticSearch(
    index_name="new_index",
    field_name="embedding",
    index_field=True,
    similarity="cosine",
)

pipeline = Pipeline(
    source=my_source,
    transform=my_transform,
    embedding=my_model,
    sink=my_sink,
    target=my_target,
)


def on_success():
    pass


def on_fail():
    pass


# Syncs the source/sink once
pipeline.pipe_real_time("", on_success, on_fail)
