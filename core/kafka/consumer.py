import json
import time

from confluent_kafka.admin import AdminClient, NewTopic
from confluent_kafka import Consumer, KafkaError, KafkaException, Message
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroDeserializer
from loguru import logger
from threading import Lock, Event

from api.config.kafka import KafkaConfig
from typing import Dict, List, Optional, Callable, Union, Any, Tuple


class KafkaConsumer:
    def __init__(self) -> None:
        self._admin = None
        self._consumer = None
        self._schema_registry_client = None
        self._topics: List[str] = []
        self._topic_primary_keys: Dict[str, Any] = {}

        self._add_topic_lock = Lock()
        self._consumer_initialize_lock = Lock()
        self._consume_records_lock = Lock()
        self._consumer_initialized = Event()

        self.is_consuming = False

    # Private Methods
    def _create_topics(self, admin: AdminClient, topics: List[str]) -> None:
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

    def _decode_message(
        self,
        encoding: str,
        message: Message,
        topic: str,
        sr_client: Optional[SchemaRegistryClient] = None,
    ) -> Tuple[str, Dict[str, Union[str, int]]]:
        key: str = ""
        value_dict: Dict[str, Union[str, int]] = {}

        try:
            if encoding == "json":
                key = message.key().decode("utf-8")
                value = message.value().decode("utf-8")
                value_dict = json.loads(value)
                return (key, value_dict)
            elif encoding == "avro":
                key = message.key().decode("utf-8")

                subject_name = f"{topic}-value"
                source_schema_str = self._return_schema(sr_client, subject_name)
                avro_deserializer = AvroDeserializer(sr_client, source_schema_str)
                value_dict = avro_deserializer(
                    message.value(),
                    SerializationContext(topic, MessageField.VALUE),
                )
        except Exception as e:
            logger.error(f"failed to decode message: {e}")

        return (key, value_dict)

    def _extract_ids(
        self, primary_key: str, records: List[Dict[str, Union[str, int]]]
    ) -> List[Union[str, int]]:
        ids = []
        for record in records:
            id_value = record.get(primary_key)
            if id_value is not None:
                ids.append(id_value)
        return ids

    def _process_messages(
        self,
        messages: List[str],
        topic: str,
        process_fn: Callable[[List[Dict[str, Any]], List[Union[str, int]]], None],
        sr_client: Optional[SchemaRegistryClient] = None,
    ) -> None:
        documents = []
        for msg in messages:
            if sr_client is not None:
                _, value = self._decode_message("avro", msg, topic, sr_client)
                documents.append(value)

        ids = self._extract_ids(self._topic_primary_keys[topic], documents)
        process_fn(documents, ids)

    def _return_schema(
        self, schema_registry_client: SchemaRegistryClient, subject_name: str
    ) -> str:
        # The result is cached so subsequent attempts will not
        # require an additional round-trip to the Schema Registry.
        return str(
            schema_registry_client.get_latest_version(subject_name).schema.schema_str
        )

    # Public Methods

    def initialize(self) -> None:
        with self._consumer_initialize_lock:
            if self._consumer_initialized.is_set():
                return

            kafka_config = KafkaConfig()
            consumer_conf = {
                "bootstrap.servers": kafka_config.bootstrap_servers,
                "group.id": "retake_kafka",
                "auto.offset.reset": "smallest",
                "enable.auto.commit": "false",
                "allow.auto.create.topics": "true",
            }

            self._consumer = Consumer(consumer_conf)
            self._admin = AdminClient(
                {"bootstrap.servers": kafka_config.bootstrap_servers}
            )
            self._schema_registry_client = SchemaRegistryClient(
                {"url": kafka_config.schema_registry_server}
            )

            self._consumer_initialized.set()

    def consume_records(
        self,
        process_fn: Callable[[List[Dict[str, Any]], List[Union[str, int]]], None],
    ) -> None:
        if self.is_consuming:
            return

        with self._consume_records_lock:
            self.is_consuming = True
            self._consumer_initialized.wait()

            if not self._consumer:
                raise Exception("Consumer not initialized in consume_records")

            BATCH_SIZE = 1000
            COMMIT_TIMEOUT = 2

            try:
                messages: List[str] = []
                start_time = time.time()
                logger.info("Starting consumer loop...")

                while True:
                    msg = self._consumer.poll(timeout=1.0)

                    if msg is None:
                        if (
                            len(messages) <= BATCH_SIZE
                            and (time.time() - start_time) >= COMMIT_TIMEOUT
                            and len(messages) > 0
                        ):
                            logger.info(
                                f"reached commit timeout. Commiting {len(messages)} messages."
                            )
                            self._consumer.commit(asynchronous=False)
                            for topic in self._topics:
                                self._process_messages(
                                    [
                                        message
                                        for message in messages
                                        if message.topic() == topic
                                    ],
                                    topic,
                                    process_fn,
                                    self._schema_registry_client,
                                )
                            messages = []
                        start_time = time.time()
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
                        else:
                            logger.info(
                                f"reached commit batch limit. Commiting {len(messages)} messages."
                            )
                            self._consumer.commit(asynchronous=False)
                            for topic in self._topics:
                                self._process_messages(
                                    [
                                        message
                                        for message in messages
                                        if message.topic() == topic
                                    ],
                                    topic,
                                    process_fn,
                                    self._schema_registry_client,
                                )
                            messages = []
            finally:
                # Close down consumer to commit final offsets.
                self._consumer.close()
                self.is_consuming = False

    def add_topic(self, topic: str, primary_key: str) -> None:
        self._consumer_initialized.wait()

        if not self._consumer:
            raise Exception("Consumer not initialized in consume_records")

        with self._add_topic_lock:
            if topic not in self._topics:
                # Create topic
                self._create_topics(self._admin, [topic])
                self._topics.append(topic)
                self._topic_primary_keys[topic] = primary_key

                # Subscribe to topics
                self._consumer.unsubscribe()
                self._consumer.subscribe(self._topics)
