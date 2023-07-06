import os
import pickle
from dotenv import load_dotenv
from ksqldb import KSQLdbClient
from retake.embedding import OpenAIEmbedding, SentenceTransformerEmbedding
from retake.source import PostgresSource
from retake.transform import PostgresTransform
from retake.sink import ElasticSearchSink

load_dotenv()
client = KSQLdbClient("http://localhost:8088")

Transform = Union[PostgresTransform]
Embedding = Union[OpenAIEmbedding, SentenceTransformerEmbedding]
Sink = Union[ElasticSearchSink]
Target = Union[ElasticSearchTarget]


def create_pg_connector(schema_name: str, relation: str):
    name = relation.lower() + "Reader"
    query = f"""CREATE SOURCE CONNECTOR {name} WITH (
     'connector.class' = 'io.debezium.connector.postgresql.PostgresConnector',
     'plugin.name' = 'pgoutput',
     'database.hostname' = '{os.environ['DB_HOST']}',
     'database.port' = '{os.environ['DB_PORT']}',
     'database.user' = '{os.environ['DB_USER']}',
     'database.password' = '{os.environ['DB_PASSWORD']}',
     'database.dbname' = '{os.environ['DB_NAME']}',
     'table.include.list' = '{schema_name}.{relation}',
     'transforms' = 'unwrap',
     'transforms.unwrap.type' = 'io.debezium.transforms.ExtractNewRecordState',
     'transforms.unwrap.drop.tombstones' = 'false',
     'transforms.unwrap.delete.handling.mode' = 'rewrite',
     'topic.prefix' = '{relation}'
 );"""
    client.ksql(query)

    stream_query = f"""CREATE STREAM {relation} WITH (
     kafka_topic = '{relation}.{schema_name}.{relation}',
     value_format = 'avro'
     );"""
    client.ksql(stream_query)


def create_collection_stream(
    schema_name: str,
    relation: str,
    embedding: Embedding,
    transform: Transform,
    sink: Sink,
    target: Target,
):
    pickled_embedding = pickle.dumps(embedding)
    pickled_transform = pickle.dumps(transform)
    pickled_sink = pickle.dumps(sink)
    pickled_target = pickle.dumps(target)
    
