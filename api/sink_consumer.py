import json
import requests
import time
from confluent_kafka.admin import AdminClient, NewTopic
from confluent_kafka import Producer, Consumer, KafkaError, KafkaException, Message
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroDeserializer
from loguru import logger

from api.config.kafka import KafkaConfig
from typing import Dict, List, Optional, Callable, Any, Tuple


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
            logger.warning("Topic {} created by sink consumer".format(topic))
        except Exception as e:
            logger.error("Failed to create topic {}: {}".format(topic, e))


def produce_message(producer: Producer, topic: str, key: str, value: str) -> None:
    producer.produce(topic=topic, key=key, value=value)
    producer.flush()


def decode_message(
    encoding: str,
    message: Message,
    topic: str,
    sr_client: Optional[SchemaRegistryClient] = None,
) -> Tuple[str, Dict[str, str]]:
    key: str = ""
    value_dict: Dict[str, str] = {}

    try:
        if encoding == "json":
            key = message.key().decode("utf-8")
            value = message.value().decode("utf-8")
            value_dict = json.loads(value)
            return (key, value_dict)
        elif encoding == "avro":
            key = message.key().decode("utf-8")

            subject_name = f"{topic}-value"
            source_schema_str = return_schema(sr_client, subject_name)
            avro_deserializer = AvroDeserializer(sr_client, source_schema_str)
            value_dict = avro_deserializer(
                message.value(),
                SerializationContext(topic, MessageField.VALUE),
            )
    except Exception as e:
        logger.error(f"failed to decode message: {e}")

    return (key, value_dict)


def extract_ids(primary_key: str, records: List[Dict[str, str]]) -> List[str]:
    ids = []
    for record in records:
        id_value = record.get(primary_key)
        if id_value is not None:
            ids.append(id_value)
    return ids


def process_messages(
    messages: List[str],
    topic: str,
    primary_key: str,
    process_fn: Callable[[List[Dict[str, Any]], List[str]], None],
    sr_client: Optional[SchemaRegistryClient] = None,
) -> None:
    documents = []
    for msg in messages:
        if sr_client is not None:
            _, value = decode_message("avro", msg, topic, sr_client)
            documents.append(value)

    ids = extract_ids(primary_key, documents)
    process_fn(documents, ids)


def return_schema(
    schema_registry_client: SchemaRegistryClient, subject_name: str
) -> str:
    # The result is cached so subsequent attempts will not
    # require an additional round-trip to the Schema Registry.
    return str(
        schema_registry_client.get_latest_version(subject_name).schema.schema_str
    )


def consume_records(
    topic: str,
    primary_key: str,
    process_fn: Callable[[List[Dict[str, Any]], List[str]], None],
) -> None:
    #  Create consumer and producer
    kafka_config = KafkaConfig()
    consumer_conf = {
        "bootstrap.servers": kafka_config.bootstrap_servers,
        "group.id": "retake_kafka",
        "auto.offset.reset": "smallest",
        "enable.auto.commit": "false",
        "allow.auto.create.topics": "true",
    }
    consumer = Consumer(consumer_conf)
    producer = Producer({"bootstrap.servers": kafka_config.bootstrap_servers})
    admin = AdminClient({"bootstrap.servers": kafka_config.bootstrap_servers})

    # Create topic in case it doesn't exist
    new_topics = [topic]
    create_topics(admin, new_topics)

    subscribe_topics = [topic]

    # Create schema registry client
    sr_client = SchemaRegistryClient({"url": kafka_config.schema_registry_server})

    BATCH_SIZE = 1000
    COMMIT_TIMEOUT = 2

    try:
        consumer.subscribe(subscribe_topics)
        messages: List[str] = []
        start_time = time.time()
        logger.info("Starting consumer loop...")
        while True:
            msg = consumer.poll(timeout=1.0)

            if msg is None:
                continue

            if msg.error():
                if msg.error().code() == KafkaError._PARTITION_EOF:
                    # End of partition event
                    logger.error(
                        "%% %s [%d] reached end at offset %d\n"
                        % (msg.topic(), msg.partition(), msg.offset())
                    )
                elif msg.error():
                    raise KafkaException(msg.error())
            else:
                if len(messages) <= BATCH_SIZE:
                    messages.append(msg)

                elif len(messages) <= BATCH_SIZE and (time.time() - start_time) >= COMMIT_TIMEOUT and len(messages) > 0:
                    logger.info(
                        f"reached commit timeout. Commiting {len(messages)} messages."
                    )
                    consumer.commit(asynchronous=False)
                    process_messages(messages, topic, primary_key, process_fn, sr_client)
                    messages = []
                    continue
                else:
                    logger.info(
                        f"reached commit batch limit. Commiting {len(messages)} messages."
                    )
                    consumer.commit(asynchronous=False)
                    process_messages(
                        messages, topic, primary_key, process_fn, sr_client
                    )
                    messages = []
    finally:
        # Close down consumer to commit final offsets.
        consumer.close()
