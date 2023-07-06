from cdc.source_postgres import PostgresCDC
from flask import Flask, request

app = Flask(__name__)

def start(conn):
    try:
        print("Starting...")
        print(conn)
        cdc = PostgresCDC(
            publication_name=conn['publication_name'],
            slot_name=conn['slot_name'],
            database=conn['dbname'],
            host=conn['host'],
            user=conn['user'],
            password=conn['password'],
            port=conn['port'],
        )
        
        for event in cdc:
            print(event)

    except Exception as e:
        if e is KeyboardInterrupt:
            print("\nException caught, tearing down...")
            cdc.teardown()
        else:
            print(e)


@app.route('/source', methods=['GET', 'POST'])
def handle_source():
    if request.method == 'POST':
        data = request.get_json()
        if data is not None:
            start(data)
        return "OK"

def parse_json(data):
    # Implement your parsing logic here
    # Example: assuming data is in JSON format
    import json
    parsed_data = json.loads(data)
    return parsed_data

if __name__ == '__main__':
    app.run()