from realtime_server.config import KafkaConfig

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
