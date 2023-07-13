import requests
from confluent_kafka import Consumer

CONFIG_MESSAGE_NUM = 2
conf = {
    "bootstrap.servers": "kafka:9094",
    "group.id": "connector_consumer",
    "auto.offset.reset": "smallest",
}

consumer = Consumer(conf)
config = consumer.consume(num_messages=CONFIG_MESSAGE_NUM)

for conf in config:
    request.post("kafka-connect:8081", config)

