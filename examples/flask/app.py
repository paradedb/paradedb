import os
import ast

from dotenv import load_dotenv
from flask import Flask, jsonify
from webargs import fields
from webargs.flaskparser import use_args
from retakesearch import Client, Search

load_dotenv()

app = Flask(__name__)

client = Client("retake-test-key", "http://localhost:8000")
table_name = os.getenv("DATABASE_TABLE_NAME")
columns = ast.literal_eval(os.getenv("DATABASE_TABLE_COLUMNS"))

search_args = {
    "query": fields.Str(required=True),
}


@app.route("/search", methods=["POST"])
@use_args(search_args, location="json")
def search(args):
    query = Search().neuralQuery(args["query"], columns)
    result = client.search(table_name, query)

    return jsonify(result)


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000)
