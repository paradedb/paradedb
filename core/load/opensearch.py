from typing import List, Union, Optional, Dict, Any
from core.load.base import Loader

# TODO: Implement OpenSearch interface


class OpenSearchLoader(Loader):
    def __init__(self) -> None:
        pass

    ### Public Methods ###

    def check_and_setup_index(
        self, index_name: str, field_name: str, num_dimensions: int
    ) -> None:
        pass

    @Loader.validate
    def bulk_upsert_embeddings(
        self,
        index_name: str,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        field_name: str,
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        pass
