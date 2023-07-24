from http import HTTPStatus
from os.path import exists
import requests
import shelve
import time
from confluent_kafka import Producer
from confluent_kafka.schema_registry import SchemaRegistryClient, Schema
from kafka_connectors.kafka_config import KafkaConfig
from kafka_connectors.secrets_handler import (
    store_env_secret,
    SecretInvalidFormatError,
)
from typing import Any, Dict

kafka_config = KafkaConfig()


def setup_store():
    # For now, store state using shelve. Note that this approach might
    # change in the future as complexity grows.
    store_name = "connector.config"
    if not exists(store_name):
        d = shelve.open(store_name)
        d["source_connector_created"] = False
        d["sink_value_schema_registered"] = False
        d.close()
        print(f"Successfully created a persistent shelve store: {store_name}")
    else:
        print("Using existing store")


def process_connector_config(message_key: str, value: Dict[str, str]) -> None:
    d = shelve.open("connector.config")
    source_connector_created = d["source_connector_created"]

    if message_key == "source-connector" and not source_connector_created:
        # Extract source type
        source_type = value.get("source_type")
        value.pop("source_type")

        #  Create source connector
        create_source_connector(value)
        d["source_connector_created"] = True
    elif message_key == "sink-connector":
        # Extract sink type
        source_type = value.get("sink_type")
        value.pop("sink_type")

        # Write to env file for later use
        for k, v in value.items():
            try:
                env_key = f"{source_type.upper()}_{k.upper()}"
                store_env_secret(env_key, v)
            except SecretInvalidFormatError:
                print("Invalid configuration received for connector")
    d.close()


def create_connector(connector_config: dict[str, Any]) -> None:
    max_retries = 15
    retry_count = 0
    while retry_count < max_retries:
        try:
            time.sleep(1)  # Wait for 1 second before retrying
            url = f"{kafka_config.connect_server}/connectors"
            r = requests.post(
                url,
                json=connector_config,
            )
            if r.status_code == HTTPStatus.OK or r.status_code == HTTPStatus.CREATED:
                print("Connector successfully created")
                break
            elif r.status_code == HTTPStatus.CONFLICT:
                print("Connector already exists")
                break
            else:
                print(r.json())
                raise requests.exceptions.RequestException(
                    f"Failed to create connector: {r.reason}"
                )
        except requests.exceptions.ConnectionError:
            print("Kafka connect server is not yet available, retrying...")
            retry_count += 1
            continue


def create_source_connector(source_type: str, conf: Dict[str, str]):
    if not source_type:
        raise ValueError("No source connector type defined")

    print(f"Creating connector for source: {source_type}")
    if source_type == "postgresql":
        create_connector(get_postgres_connector_config(conf))


def produce_config_ready_message(producer: Producer, status: str) -> None:
    # Once the schema is registered, produce a special message to the readiness topic
    producer.produce(topic="_config_success", key="config_ready", value=status)
    producer.flush()
    print("Produced to config ready topic")


def get_postgres_connector_config(conf: Dict[str, str]) -> Dict:
    return {
        "name": f'{conf["table_name"]}-connector',
        "config": {
            "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
            "plugin.name": "pgoutput",
            "value.converter": "io.confluent.connect.avro.AvroConverter",
            "value.converter.schema.registry.url": kafka_config.schema_registry_server,
            "database.hostname": f'{conf["host"]}',
            "database.port": f'{conf["port"]}',
            "database.user": f'{conf["user"]}',
            "database.password": f'{conf["password"]}',
            "database.dbname": f'{conf["dbname"]}',
            "table.include.list": f'{conf["schema_name"]}.{conf["table_name"]}',
            "transforms": "unwrap",
            "transforms.unwrap.type": "io.debezium.transforms.ExtractNewRecordState",
            "transforms.unwrap.drop.tombstones": "false",
            "transforms.unwrap.delete.handling.mode": "rewrite",
            "topic.prefix": f'{conf["table_name"]}',
        },
    }


def register_sink_value_schema() -> None:
    print("registering sink schema...")
    d = shelve.open("connector.config")
    sink_value_schema_registered = d["sink_value_schema_registered"]

    schema_str = """
    {
    "name": "embeddings",
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
        },
        {
            "name": "sink",
            "type": "string",
            "default": "value"
        },
        {
            "name": "field_name",
            "type": "string",
            "default": "value"
        },
        {
            "name": "index_name",
            "type": "string",
            "default": "value"
        }
    ]
    }
    """

    if not sink_value_schema_registered:
        while True:
            try:
                time.sleep(1)  # Wait for 1 second before retrying
                avro_schema = Schema(schema_str, "AVRO")
                sr = SchemaRegistryClient({"url": kafka_config.schema_registry_server})
                subject_name = "embeddings-value"
                sr.register_schema(subject_name, avro_schema)
                break

            except requests.exceptions.ConnectionError:
                print("Schema registry server is not yet available, retrying...")

        print("Schema written successfully")
        d["sink_value_schema_registered"] = True

    d.close()
