import json
import requests
from confluent_kafka.admin import AdminClient, NewTopic
from confluent_kafka import Consumer, KafkaException
from connectors.config import KafkaConfig
from connectors.connect import create_source_connector, register_sink_value_schema, create_sink_connector

kafka_config = KafkaConfig()

consumer_conf = {
    "bootstrap.servers": kafka_config.bootstrap_servers,
    "group.id": "connector_consumer",
    "auto.offset.reset": "smallest",
    "allow.auto.create.topics": "true"
}

topics = ["_connector-config"]
a = AdminClient({'bootstrap.servers': kafka_config.bootstrap_servers})

# Create topic
new_topics = [NewTopic(topic, num_partitions=3, replication_factor=1) for topic in topics]
fs = a.create_topics(new_topics)

# Wait for each operation to finish.
for topic, f in fs.items():
    try:
        f.result()  # The result itself is None
        print("Topic {} created".format(topic))
    except Exception as e:
        print("Failed to create topic {}: {}".format(topic, e))

c = Consumer(consumer_conf)

# Subscribe to topics
c.subscribe(topics)

# Read messages from Kafka, print to stdout
done = False
while not done:
    msg = c.poll(1.0)

    if msg is None:
        continue
    if msg.error():
        print("Consumer error: {}".format(msg.error()))
        continue

    key = msg.key().decode('utf-8')
    value = msg.value().decode('utf-8')
    value_dict = json.loads(value)

    print(value_dict)
    if key == "source-connector":
        create_source_connector(value_dict)
    else:
        if "index" in value_dict:
            create_sink_connector(value_dict)
            register_sink_value_schema(value_dict["index"])
        else:
            raise Exception("no index found")

c.close()