from abc import ABC, abstractmethod


class Extractor(ABC):
    @abstractmethod
    def extract_all(
        self, relation: str, columns: str, primary_key: str, chunk_size: int
    ):
        pass

    @abstractmethod
    def count(self, relation: str):
        pass

    @abstractmethod
    def teardown(self):
        pass
