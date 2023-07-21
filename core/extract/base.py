from abc import ABC, abstractmethod
from typing import Generator, TypedDict, Union, List, Any


class ExtractorResult(TypedDict):
    rows: List[Any]
    primary_keys: List[Union[str, int]]


class Extractor(ABC):
    @abstractmethod
    def extract_all(
        self, relation: str, columns: List[str], primary_key: str, chunk_size: int
    ) -> Generator[ExtractorResult, None, None]:
        pass

    @abstractmethod
    def count(self, relation: str) -> int:
        pass

    @abstractmethod
    def teardown(self) -> None:
        pass
