from http import HTTPStatus
import requests
import time
from confluent_kafka.schema_registry import SchemaRegistryClient, Schema
from kafka_connectors.kafka_config import KafkaConfig
from typing import Any, Dict

kafka_config = KafkaConfig()


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


def create_source_connector(conf: Dict[str, str]):
    if conf["type"] == "postgres":
        create_connector(get_postgres_connector_config())


def create_sink_connector(conf: Dict[str, str]):
    create_connector(get_http_sink_config())


def get_postgres_connector_config() -> None:
    print("creating postgres source connector...")
    create_connector(
        {
            "name": f'{conn["table_name"]}-connector',
            "config": {
                "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
                "plugin.name": "pgoutput",
                "value.converter": "io.confluent.connect.avro.AvroConverter",
                "value.converter.schema.registry.url": kafka_config.schema_registry_server,
                "database.hostname": f'{conn["host"]}',
                "database.port": f'{conn["port"]}',
                "database.user": f'{conn["user"]}',
                "database.password": f'{conn["password"]}',
                "database.dbname": f'{conn["dbname"]}',
                "table.include.list": f'{conn["schema_name"]}.{conn["table_name"]}',
                "transforms": "unwrap",
                "transforms.unwrap.type": "io.debezium.transforms.ExtractNewRecordState",
                "transforms.unwrap.drop.tombstones": "false",
                "transforms.unwrap.delete.handling.mode": "rewrite",
                "topic.prefix": f'{conn["table_name"]}',
            },
        }
    )


def register_sink_value_schema(index: str) -> None:
    print("registering sink schema...")
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

    while True:
        try:
            time.sleep(1)  # Wait for 1 second before retrying
            avro_schema = Schema(schema_str, "AVRO")
            sr = SchemaRegistryClient({"url": kafka_config.schema_registry_server})
            subject_name = f"{index}-value"
            sr.register_schema(subject_name, avro_schema)
            break

        except requests.exceptions.ConnectionError:
            print("Schema registry server is not yet available, retrying...")

    print("Schema written successfully")
