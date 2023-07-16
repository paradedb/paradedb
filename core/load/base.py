from abc import ABC, abstractmethod
from functools import wraps
from typing import Dict, List, Union, Callable, Any, Optional, TypeVar, cast

T = TypeVar("T")


class Loader(ABC):
    @staticmethod
    def validate(func: Callable[..., T]) -> Callable[..., T]:
        @wraps(func)
        def wrapper(self, *args: Any, **kwargs: Any) -> Any:  # type: ignore
            embeddings = cast(List[List[float]], kwargs.get("embeddings"))
            ids = cast(List[Union[str, int]], kwargs.get("ids"))
            metadata = cast(
                Optional[List[Dict[str, Any]]], kwargs.get("metadata", None)
            )

            num_dimensions = len(embeddings[0])
            num_embeddings = len(embeddings)

            if not all(len(embedding) == num_dimensions for embedding in embeddings):
                raise ValueError(
                    "Not all embeddings have the same number of dimensions"
                )

            if not len(ids) == num_embeddings:
                raise ValueError("Number of ids does not match number of embeddings")

            if metadata is not None and not len(ids) == len(metadata):
                raise ValueError("Number of metadata does not match number of ids")

            return func(self, *args, **kwargs)

        return wrapper

    @abstractmethod
    def check_and_setup_index(self, *args: Any, **kwargs: Any) -> None:
        pass

    @abstractmethod
    def bulk_upsert_embeddings(self, *args: Any, **kwargs: Any) -> None:
        pass
