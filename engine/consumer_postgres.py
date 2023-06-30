from typing import Dict, List
from pydantic import BaseModel

from engine.pgoutput import Relation, Update, Insert, Delete


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
        decoded_message = Update(message.payload)
        relation_id = decoded_message.relation_id

        old_dict = self._format_tuple(decoded_message.old_tuple, relation_id)
        new_dict = self._format_tuple(decoded_message.new_tuple, relation_id)

        diff_dict = self._diff_dicts(old_dict, new_dict)

        self.queue.put({"type": PGOutput.UPDATE, "message": diff_dict})

    def _handle_insert(self, message):
        # TODO (Phil) - On INSERT, emit the inserted row (see handle_update)
        # Insert() class already written in pgoutput.py
        self.queue.put({"type": PGOutput.INSERT, "message": "decoded_message_here"})

    def _handle_delete(self, message):
        # TODO (Phil) - On DELETE, emit the deleted row (see handle_update)
        # Delete() class already written in pgoutput.py
        self.queue.put({"type": PGOutput.DELETE, "message": "decoded_message_here"})

    def _format_tuple(self, tuple_data, relation_id):
        table_schema = self.table_schemas[relation_id]
        output = dict()

        for index, column in enumerate(tuple_data.column_data):
            column_name = table_schema.columns[index]
            output[column_name] = column.col_data

        return output

    def _diff_dicts(self, dict1, dict2):
        before = dict()
        after = dict()

        for key in dict1.keys() & dict2.keys():
            if dict1[key] != dict2[key]:
                before[key] = dict1[key]
                after[key] = dict2[key]

        return {"before": before, "after": after}
