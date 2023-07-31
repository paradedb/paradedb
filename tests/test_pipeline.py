import psycopg2

from clients.python.retakesearch import Table, Database
from clients.python.retakesearch.search import Search


def test_postgres_to_opensearch(
    postgresql,
    retake_client,
    test_table_name,
    test_primary_key,
    test_column_name,
    test_index_name,
    test_document_id
):
    # Initialize a temporary database and associated table
    temp_conn = psycopg2.connect(
        host=postgresql.info.host,
        user=postgresql.info.user,
        password=postgresql.info.password,
        port=postgresql.info.port,
        dbname=postgresql.info.dbname,
    )

    with temp_conn.cursor() as cursor:
        cursor.execute(
            f"CREATE TABLE {test_table_name} ({test_primary_key} varchar PRIMARY KEY, {test_column_name} varchar);"
        )
        cursor.execute(
            f"INSERT INTO {test_table_name} VALUES ('{test_document_id}', 'fake_data1'), ('id2', 'fake_data2'), ('id3', 'fake_data3');"
        )
        temp_conn.commit()


    print("CONNECTED TO POSTGRES")
    print(temp_conn)

    # Return Source
    database = Database(
        host=postgresql.info.host,
        user=postgresql.info.user,
        password=postgresql.info.password,
        port=postgresql.info.port,
        dbname=postgresql.info.dbname
    )

    # Initialize a temporary database and associated table
    table = Table(
        name=test_table_name,
        primary_key=test_primary_key,
        columns=[test_column_name],
        neural_columns=[test_column_name],
    )

    # Create an index for our vectors in OpenSearch, and sync the database table to it
    index = retake_client.create_index(test_index_name)
    index.add_source(database, table)

    # Test that the data was loaded and can be searched
    bm25_search_query = Search().query("match_all")
    response = index.search(bm25_search_query)
    print(response)

    neural_search_query = Search().neuralQuery("fake data", [test_column_name])
    response = index.search(neural_search_query)
    print(response)

    temp_conn.close()
