import os
import ast
import json

from dotenv import load_dotenv
from flask import Flask, Response
from webargs import fields
from webargs.flaskparser import use_args
from typing import Dict, Any

from retakesearch import Client, Search

load_dotenv()

app = Flask(__name__)

client = Client("retake-test-key", "http://localhost:8000")
table_name = os.getenv("DATABASE_TABLE_NAME")
columns = ast.literal_eval(os.getenv("DATABASE_TABLE_COLUMNS", "[]"))

search_args = {
    "query": fields.Str(required=True),
}


@app.route("/search", methods=["POST"])
@use_args(search_args, location="json")  # type: ignore
def search(args: Dict[str, Any]) -> Response:
    if not table_name or not columns:
        return Response(
            status=400,
            response="Table name or columns is empty. Check DATABASE_TABLE_NAME and DATABASE_TABLE_COLUMNS environment variables.",
        )

    index = client.get_index(table_name)

    if not index:
        return Response(
            status=400,
            response=f"Table {table_name} was not indexed. Did you run scripts/setup.py?",
        )

    query = Search().with_neural(args["query"], columns)
    result = index.search(query)

    return Response(status=200, response=json.dumps(result))


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000)
