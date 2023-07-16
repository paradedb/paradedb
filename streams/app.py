import json
import socket
from core.sdk.realtime import RealtimeServer
from core.sdk.source import PostgresSource
from core.sdk.sink import ElasticSearchSink
from confluent_kafka import Producer, Consumer
from confluent_kafka.serialization import SerializationContext, MessageField
from confluent_kafka.schema_registry import SchemaRegistryClient
from confluent_kafka.schema_registry.avro import AvroSerializer
from faust import App, Worker
from typing import Callable, Any, Optional, Union

Source = Union[PostgresSource]
Sink = Union[ElasticSearchSink]


def return_schema(
    schema_registry_client: SchemaRegistryClient, subject_name: str
) -> str:
    # The result is cached so subsequent attempts will not
    # require an additional round-trip to the Schema Registry.
    return str(
        schema_registry_client.get_latest_version(subject_name).schema.schema_str
    )


def register_connector_conf(
    server: RealtimeServer,
    index: str,
    db_schema_name: str,
    table_name: str,
    source: Source,
    sink: Sink,
):
    config_topic = "_connector_config"
    conf = {"bootstrap.servers": server.broker_host, "client.id": socket.gethostname()}

    # Append table name and schema name to source config
    source_conf = source.config
    source_conf["schema_name"] = db_schema_name
    source_conf["table_name"] = table_name

    # Append index to sink config
    sink_conf = sink.config
    sink_conf["index"] = index

    p = Producer(conf)
    try:
        p.produce(config_topic, key="source-connector", value=json.dumps(source_conf))
        p.produce(config_topic, key="sink-connector", value=json.dumps(sink_conf))

    except BufferError:
        print(
            "%% Local producer queue is full (%d messages awaiting delivery): try again\n"
            % len(p)
        )
    print("%% Waiting for %d deliveries\n" % len(p))
    p.flush()


def wait_for_config_success(server: RealtimeServer):
    print("Waiting for connector configuration to be ready")
    consumer = Consumer(
        {
            "bootstrap.servers": server.broker_host,
            "group.id": "_config_success_group",
            "auto.offset.reset": "earliest",
        }
    )
    topics = ["_config_success"]
    consumer.subscribe(topics)

    while True:
        msg = consumer.poll(timeout=1.0)

        if msg is None:
            continue
        if msg.error():
            print("Consumer error: {}".format(msg.error()))
            continue

        key = msg.key().decode("utf-8")
        if key == "config_ready":
            print("Configuration is ready!")
            break

    consumer.close()


def register_agents(
    topic: str,
    index: str,
    server: RealtimeServer,
    embedding_fn: Callable[..., Any],  # TODO: proper typing
    transform_fn: Callable[..., str],
    metadata_fn: Optional[Callable[..., list[str]]],
) -> Worker:
    app = App(
        "realtime",
        broker=f"kafka://{server.broker_host}",
        value_serializer="raw",
    )
    source_topic = app.topic(topic, value_serializer="raw")
    sr_client = SchemaRegistryClient({"url": server.schema_registry_url})
    subject_name = f"{index}-value"
    schema_str = return_schema(sr_client, subject_name)
    avro_serializer = AvroSerializer(sr_client, schema_str)
    producer_conf = {"bootstrap.servers": server.broker_host}
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

                    message = {"doc": embedding, "metadata": metadata}
                    producer.produce(
                        topic=index,
                        value=avro_serializer(
                            message, SerializationContext(topic, MessageField.VALUE)
                        ),
                    )

    return Worker(app, loglevel="INFO")


def start_worker(worker: Worker) -> None:
    print("starting faust worker...")
    worker.execute_from_commandline()
