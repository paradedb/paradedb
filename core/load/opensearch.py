from typing import List

# TODO (Phil): Implement OpenSearch interface


class OpenSearch:
    def __init__(self):
        pass

    def insert(self, embedding: List[float], id: str, metadata: dict = None):
        pass

    def delete(self, id: str):
        pass

    def query(self, document: str):
        pass
