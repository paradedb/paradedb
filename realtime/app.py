import numpy
import os
import requests
import json
from asyncio import get_event_loop, ensure_future
from confluent_kafka import Producer
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient, Schema
from confluent_kafka.schema_registry.avro import AvroSerializer
from core.transform.base import Embedding
from faust import App, Worker
from nptyping import NDArray, Shape, Float32
from realtime.config import KafkaConfig
from typing import Callable, Any, Optional

kafka_config = KafkaConfig()

def create_source_connector(conn: dict[str, str]) -> None:
    try:
        url = f"{kafka_config.connect_server}/connectors"
        r = requests.post(
            url,
            json={
                "name": f'{conn["relation"]}-connector',
                "config": {
                    "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
                    "plugin.name": "pgoutput",
                    "value.converter": "org.apache.kafka.connect.json.JsonConverter",  # TODO: support avro
                    "database.hostname": f'{conn["host"]}',
                    "database.port": f'{conn["port"]}',
                    "database.user": f'{conn["user"]}',
                    "database.password": f'{conn["password"]}',
                    "database.dbname": f'{conn["dbname"]}',
                    "table.include.list": f'{conn["schema_name"]}.{conn["relation"]}',
                    "transforms": "unwrap",
                    "transforms.unwrap.type": "io.debezium.transforms.ExtractNewRecordState",
                    "transforms.unwrap.drop.tombstones": "false",
                    "transforms.unwrap.delete.handling.mode": "rewrite",
                    "topic.prefix": f'{conn["relation"]}',
                },
            },
        )
    except Exception as e:
        # TODO: handle
        print(e)


def register_sink_value_schema(index: str) -> int:
    schema_str = """
    {
    "name": "embedding",
    "type": "record",
    "fields": [
        {
        "name": "doc",
        "type": {
            "type": "array",
            "items": "float"
        }
        },
        {
        "name": "metadata",
        "type": {
            "type": "array",
            "items": "string"
        },
        "default": []
        }
    ]
    }
    """
    avro_schema = Schema(schema_str, "AVRO")
    sr = SchemaRegistryClient({"url": kafka_config.schema_registry_server})
    schema_id = sr.register_schema(f"{index}-value", avro_schema)
    return schema_id


def return_schema(schema_registry_client: SchemaRegistryClient, schema_id: int) -> str:
    # The result is cached so subsequent attempts will not
    # require an additional round-trip to the Schema Registry.
    return schema_registry_client.get_schema(schema_id).schema_str


def create_sink_connector(conn: dict[str, str]) -> None:
    try:
        url = f"{kafka_config.connect_server}/connectors"
        r = requests.post(
            url,
            json={
                "name": f"sink-connector",
                "config": {
                    "connector.class": "io.confluent.connect.elasticsearch.ElasticsearchSinkConnector",
                    "topics": f'{conn["index"]}',
                    "key.ignore": "true",
                    "name": "sink-connector",
                    "value.converter": "io.confluent.connect.avro.AvroConverter",
                    "value.converter.schema.registry.url": f"{kafka_config.schema_registry_internal_server}",
                    "connection.url": f'{conn["host"]}',
                    "connection.username": f'{conn["user"]}',
                    "connection.password": f'{conn["password"]}',
                },
            },
        )
    except Exception as e:
        # TODO: handle
        print(e)


def register_agents(
    topic: str,
    index: str,
    schema_id: int,
    embedding_fn: Callable[..., Any],  # TODO: proper typing
    transform_fn: Callable[..., str],
    metadata_fn: Optional[Callable[..., list[str]]],
) -> None:
    app = App(
        "realtime",
        broker=f"kafka://{kafka_config.bootstrap_servers}",
        value_serializer="raw",
    )
    source_topic = app.topic(topic, value_serializer="raw")
    sr_client = SchemaRegistryClient({"url": kafka_config.schema_registry_server})
    schema_str = return_schema(sr_client, schema_id)
    avro_serializer = AvroSerializer(sr_client, schema_str)
    producer_conf = {"bootstrap.servers": kafka_config.bootstrap_servers}
    producer = Producer(producer_conf)

    @app.agent(source_topic)
    async def process_records(records: Any) -> None:
        async for record in records:
            if record is not None:
                data = json.loads(record)
                payload = data["payload"]
                print(payload)
                if payload["__deleted"] == "true":
                    print("record was deleted, removing embedding...")
                else:
                    # TODO: Make distinction when update or new record
                    payload.pop("__deleted")
                    document = transform_fn(*payload)
                    embedding = embedding_fn(document)

                    metadata = []
                    if metadata_fn is not None:
                        metadata = metadata_fn(*payload)

                    message = {"doc": embedding.tolist(), "metadata": metadata}
                    producer.produce(
                        topic=index,
                        value=avro_serializer(
                            message, SerializationContext(topic, MessageField.VALUE)
                        ),
                    )


def start_worker() -> None:
    print("starting faust worker...")
    worker = Worker(app, loglevel="INFO")
    worker.execute_from_commandline()
