from pgoutput import Relation, Update, Insert, Delete
from typing import Dict, List
from pydantic import BaseModel


class PGOutput:
    RELATION = "R"
    UPDATE = "U"
    INSERT = "I"
    DELETE = "D"


class TableSchema(BaseModel):
    relation_id: int
    table_name: str
    columns: List[str]


class PostgresConsumer(object):
    def __init__(self, queue):
        self.queue = queue

        # Initialize dict that stores table schemas
        self.table_schemas = dict()

    def __call__(self, message):
        message_type = self._classify_message(message)

        if message_type == PGOutput.RELATION:
            self._handle_relation(message)

        if message_type == PGOutput.UPDATE:
            self._handle_update(message)

        if message_type == PGOutput.INSERT:
            self._handle_insert(message)

        if message_type == PGOutput.DELETE:
            self._handle_delete(message)

    def _classify_message(self, message):
        return message.payload[:1].decode("utf-8")

    def _handle_relation(self, message):
        decoded_message = Relation(message.payload)
        relation_id = decoded_message.relation_id
        self.table_schemas[relation_id] = TableSchema(
            relation_id=decoded_message.relation_id,
            table_name=decoded_message.relation_name,
            columns=[column.name for column in decoded_message.columns],
        )

    def _handle_update(self, message):
        self.queue.put({"type": PGOutput.UPDATE, "message": "decoded_message_here"})

    def _handle_insert(self, message):
        self.queue.put({"type": PGOutput.INSERT, "message": "decoded_message_here"})

    def _handle_delete(self, message):
        self.queue.put({"type": PGOutput.DELETE, "message": "decoded_message_here"})
