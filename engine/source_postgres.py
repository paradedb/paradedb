import psycopg2
import select
import json
import threading
import queue

from psycopg2.extras import LogicalReplicationConnection
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from engine.consumer_postgres import PostgresConsumer


class PostgresCDC:
    def __init__(
        self, publication_name, slot_name, database, host, user, password, port
    ):
        # Store Postgres connection info
        self.publication_name = publication_name
        self.slot_name = slot_name
        self.dsn = (
            f"dbname={database} user={user} password={password} host={host} port={port}"
        )

        # Queue to store decoded messages waiting to be emitted by __next__
        self.decoded_message_queue = queue.Queue()
        # Initialize consumer that decodes messages
        self.consumer = PostgresConsumer(self.decoded_message_queue)

        # Perform setup tasks in the main process
        self._connect()
        self._create_publication()
        self._create_slot()
        self._set_replica_identity()
        self._start_replication()

        # Start the _watch method in a separate process
        self.watch_thread = threading.Thread(target=self._watch)
        self.watch_thread.daemon = True  # Ensure thread exits when main program exits
        self.watch_thread.start()

    ### Private Methods ###

    def _connect(self):
        self.connection = psycopg2.connect(
            self.dsn, connection_factory=LogicalReplicationConnection
        )
        self.cursor = self.connection.cursor()

    def _create_publication(self):
        self.cursor.execute(
            f"SELECT * FROM pg_publication WHERE pubname = '{self.publication_name}'"
        )
        if not self.cursor.fetchone():
            self.cursor.execute(
                f"CREATE PUBLICATION {self.publication_name} FOR ALL TABLES"
            )
            self.connection.commit()

    def _create_slot(self):
        self.cursor.execute(
            f"SELECT * FROM pg_replication_slots WHERE slot_name = '{self.slot_name}'"
        )
        if not self.cursor.fetchone():
            self.cursor.execute(
                f"SELECT * FROM pg_create_logical_replication_slot('{self.slot_name}', 'pgoutput')"
            )
            self.connection.commit()

    def _set_replica_identity(self):
        self.cursor.execute(
            "ALTER TABLE public.ecoinvent_with_types REPLICA IDENTITY FULL"
        )

    def _start_replication(self):
        options = {"proto_version": "1", "publication_names": self.publication_name}
        output_plugin = "pgoutput"

        try:
            self.cursor.start_replication(slot_name=self.slot_name, options=options)
        except psycopg2.ProgrammingError:
            self.cursor.create_replication_slot(
                self.slot_name, output_plugin=output_plugin
            )
            self.cursor.start_replication(slot_name=self.slot_name, options=options)

    def _watch(self):
        self.cursor.consume_stream(self.consumer)

    ### Public Methods ###

    def teardown(self):
        # Stop replication connection
        self.cursor.close()
        self.connection.close()

        # Create a new connection to drop replication slot
        temp_conn = psycopg2.connect(self.dsn)
        temp_cursor = temp_conn.cursor()
        temp_cursor.execute(f"SELECT pg_drop_replication_slot('{self.slot_name}');")
        temp_cursor.close()
        temp_conn.close()

        print("Replication slot dropped")

    def __iter__(self):
        return self

    def __next__(self):
        return self.decoded_message_queue.get(block=True)
