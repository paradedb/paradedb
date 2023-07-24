import json
import requests
from confluent_kafka.admin import AdminClient, NewTopic
from confluent_kafka import Producer, Consumer, KafkaException, Message
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroDeserializer
from kafka_connectors.kafka_config import KafkaConfig
from kafka_connectors.connector_config import (
    process_connector_config,
    setup_store,
    register_sink_value_schema,
)
from kafka_connectors.sink_handler import load
from typing import Dict, List, Optional


def create_topics(admin: AdminClient, topics: List[str]) -> None:
    # Create topic
    new_topics = [
        NewTopic(topic, num_partitions=3, replication_factor=1) for topic in topics
    ]
    fs = admin.create_topics(new_topics)

    # Wait for each operation to finish.
    for topic, f in fs.items():
        try:
            f.result()  # The result itself is None
            print("Topic {} created".format(topic))
        except Exception as e:
            print("Failed to create topic {}: {}".format(topic, e))


def produce_message(producer: Producer, topic: str, key: str, value: str) -> None:
    producer.produce(topic=topic, key=key, value=value)
    producer.flush()


def decode_message(
    encoding: str, message: Message, sr_client: Optional[SchemaRegistryClient] = None
) -> (str, Dict[str, str]):
    if encoding == "json":
        key = message.key().decode("utf-8")
        value = message.value().decode("utf-8")
        value_dict = json.loads(value)
        return (key, value_dict)
    elif encoding == "avro":
        key = message.key().decode("utf-8")

        subject_name = "embeddings-value"
        source_schema_str = return_schema(sr_client, subject_name)
        avro_deserializer = AvroDeserializer(sr_client, source_schema_str)
        value_dict = avro_deserializer(
            message.value(),
            SerializationContext(source_topic, MessageField.VALUE),
        )

        return (key, value_dict)


def return_schema(
    schema_registry_client: SchemaRegistryClient, subject_name: str
) -> str:
    # The result is cached so subsequent attempts will not
    # require an additional round-trip to the Schema Registry.
    return str(
        schema_registry_client.get_latest_version(subject_name).schema.schema_str
    )


def main() -> None:
    #  Create consumer, producer and admin client
    kafka_config = KafkaConfig()
    consumer_conf = {
        "bootstrap.servers": kafka_config.bootstrap_servers,
        "group.id": "retake_kafka",
        "auto.offset.reset": "smallest",
        "allow.auto.create.topics": "true",
    }
    consumer = Consumer(consumer_conf)
    producer = Producer({"bootstrap.servers": kafka_config.bootstrap_servers})
    admin = AdminClient({"bootstrap.servers": kafka_config.bootstrap_servers})

    new_topics = ["_connector_config", "_config_success", "embeddings"]
    create_topics(admin, new_topics)

    # Subscribe to topics
    subscribe_topics = ["_connector_config", "embeddings"]
    consumer.subscribe(subscribe_topics)

    # Create schema registry client
    sr_client = SchemaRegistryClient({"url": kafka_config.schema_registry_server})

    # Create state
    setup_store()

    # Register sink schema
    register_sink_value_schema()

    while True:
        msg = consumer.poll()
        topic, partition = msg.topic(), msg.partition()

        if msg is None:
            continue
        if msg.error():
            print("Consumer error: {}".format(msg.error()))
            continue

        if topic == "_connector_config":
            key, value = decode_message("json", msg)
            try:
                process_connector_config(key, value)
                produce_message(producer, "_config_success", "config_ready", "true")
            except requests.exceptions.RequestException as e:
                produce_message(producer, "_config_success", "config_ready", "false")

        elif topic == "embeddings":
            key, value = decode_message("avro", msg, sr_client)
            try:
                load(key, value)
            except Exception as e:
                print(e)
                print("Failed to load event")
                print(key, value)

    consumer.close()


if __name__ == "__main__":
    main()
