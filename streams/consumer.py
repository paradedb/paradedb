from confluent_kafka import Consumer

conf = {
    "bootstrap.servers": "kafka:9094",
    "group.id": "connector_consumer",
    "auto.offset.reset": "smallest",
}

consumer = Consumer(conf)
consumer.consume()

