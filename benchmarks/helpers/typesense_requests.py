#!/usr/bin/env python3

import requests
import json
import sys

BASE_URL = "http://localhost:8108"
API_KEY = "xyz"
HEADERS = {
    "Content-Type": "application/json",
    "X-TYPESENSE-API-KEY": API_KEY
}

def collection_exists():
    response = requests.get(f"{BASE_URL}/collections/wikipedia_articles", headers=HEADERS)
    return 1 if response.status_code == 200 else 0

def create_collection():
    data = {
        "name": "wikipedia_articles",
        "fields": [
            {"name": "title", "type": "string"},
            {"name": "body", "type": "string"},
            {"name": "url", "type": "string"}
        ]
    }
    requests.post(f"{BASE_URL}/collections", headers=HEADERS, json=data)

def bulk_import(data_filename):
    try:
        with open(data_filename, 'r', encoding='utf-8') as f:
            data = f.read().encode('utf-8')
        response = requests.post(f"{BASE_URL}/collections/wikipedia_articles/documents/import?batch_size=500", headers=HEADERS, data=data)
        response.raise_for_status()  # This will raise an HTTPError if the HTTP request returned an error
    except requests.RequestException as e:
        print(f"Error in bulk_import: {e}")
        sys.exit(1)

def search():
    params = {
        "q": "Canada",
        "query_by": "title,body"
    }
    requests.get(f"{BASE_URL}/collections/wikipedia_articles/documents/search", headers=HEADERS, params=params)

def num_documents():
    response = requests.get(f"{BASE_URL}/collections/wikipedia_articles", headers=HEADERS)
    return json.loads(response.text)['num_documents']

if __name__ == "__main__":
    action = sys.argv[1]
    if action == "collection_exists":
        print(collection_exists())
    elif action == "create_collection":
        create_collection()
    elif action == "bulk_import":
        bulk_import(sys.argv[2])
    elif action == "search":
        search()
    elif action == "num_documents":
        print(num_documents())
