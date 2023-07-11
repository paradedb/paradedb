from retake import Source, Transform, Embedding, Sink, Target, Pipeline
from typing import Union, Dict

# Replace with your database connection credentials
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
    # In this example, Retake uses the primary key as the
    # ElasticSearch document ID by default
    primary_key="factor_id",
    relation="ecoinvent_with_types",
    columns=["activity_name", "reference_product_name"],
    transform_func=transform_func,
    optional_metadata=optional_metadata,
)

model = Embedding.SentenceTransformer(model="all-MiniLM-L6-v2")

# Replace with your sink connection credentials
# This works for Elastic Cloud
# sink = Sink.ElasticSearch(
#     user="elastic",
#     password="DZBP2fvzCnJ3t9RVMAXs3Jy8",
#     cloud_id="ea420955c7084396a384fdc14586eb0b:dXMtY2VudHJhbDEuZ2NwLmNsb3VkLmVzLmlvJGRhMzcyMmRiZGY1YzRiMWZhYmVmODlhNTVhYTk0MDU2JDg3MWVkYjRmNTZjNjQ5MjU5NTVhYzc1MGY5YjMwZWM2"
# )

sink = Sink.Weaviate(
    url="https://test2-rp8gqk80.weaviate.network",  # URL of your Weaviate instance
    api_key="cbWr4ACjcp7Kgwiw5rpbIwQUjZNqdS9Gm0Jz",
    # auth_client_secret=auth_config,  # (Optional) If the Weaviate instance requires authentication
    # timeout_config=(
    #     5,
    #     15,
    # ),  # (Optional) Set connection timeout & read timeout time in seconds
    # additional_headers={  # (Optional) Any additional headers; e.g. keys for API inference services
    #     "X-Cohere-Api-Key": "YOUR-COHERE-API-KEY",  # Replace with your Cohere key
    #     "X-HuggingFace-Api-Key": "YOUR-HUGGINGFACE-API-KEY",  # Replace with your Hugging Face key
    #     "X-OpenAI-Api-Key": "YOUR-OPENAI-API-KEY",  # Replace with your OpenAI key
    # },
)


# This works for Elastic Cloud
# target = Target.ElasticSearch(
#     index_name="faqs",
#     field_name="question_answer_embedding",
#     # Note: If an index with index_name does not already exist
#     # in ElasticSearch, a new index will be created with the below
#     # properties for field_name. If the index already exists,
#     # these properties will be ignored.
#     should_index=True,
#     similarity="cosine",
# )
target = Target.Weaviate(
    index_name="faqs",
    field_name="question_answer_embedding",
)


pipeline = Pipeline(
    source=source,
    transform=transform,
    embedding=model,
    sink=sink,
    target=target,
)

# This could take a while, depending on your table size and embedding model
pipeline.pipe_once(verbose=True)
