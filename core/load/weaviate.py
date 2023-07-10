import weaviate

from abc import ABC, abstractmethod
from core.load.base import Loader
from typing import Dict, List, Union, Optional, Any, cast
from core.sdk.target import WeaviateTarget


class WeaviateLoader(Loader):
    def __init__(
        self,
        api_key: str,
        url: str,
    ) -> None:
        self.wc = weaviate.Client(
            url=url,
            auth_client_secret=weaviate.AuthApiKey(api_key=api_key),
        )





    # In Weaviate, an index is called an object
    def _check_index_exists(self, index_name: str) -> bool:
        # try:
        #     weaviate.Client.data_object.exists(id, class_name=index_name)
        #     return True
        # except weaviate.NotFoundException as e:
        #     return False
        pass

    def _get_num_dimensions(self, index_name: str) -> int:
        pass

    def _create_index(self, index_name: str, num_dimensions: int) -> None:
        pass

    def check_and_setup_index(
        self, target: WeaviateTarget, num_dimensions: int
    ) -> None:
        pass

    # A class is the equivalent of an index in Weaviate
    # def _check_class_exists(self, class_name: str) -> bool:
    #     schema = self.wc.get_schema()
    #     classes = schema["classes"]
    #     return any(c["class"] == class_name for c in classes)

    # def _create_class(
    #     self, class_name: str, field_name: str, num_dimensions: int
    # ) -> None:
    #     if self._check_class_exists(class_name=class_name):
    #         raise ValueError(f"Class {class_name} already exists")

    # vector_schema = cast(
    #     Dict[str, Any],
    #     {
    #         "class": class_name,
    #         "properties": {
    #             field_name: {
    #                 "dataType": ["float"],
    #                 "vectorIndexType": "hnsw",
    #                 "embeddingDimensions": num_dimensions,
    #             }
    #         },
    #     },
    # )

    # self.wc.schema.create(vector_schema)

    ### Public Methods ###

    def upsert_embedding(
        self,
        target: WeaviateTarget,
        embedding: List[float],
        id: Union[str, int],
        metadata: Optional[Dict[str, Any]],
    ) -> None:
    
        # This will create the class if it doesn't exist, otherwise it will do nothing
        # self._create_class(
        #     class_name=index_name,
        #     field_name=field_name,
        #     num_dimensions=len(embedding),
        # )

        # vector_data = [
        #     {"class": index_name, "properties": {field_name: embedding}},
        # ]

        # # Weaviate only thinks in terms of batch, so here we have a batch of one
        # self.wc.batch.create_batch(vector_data)
        print("here")
        pass

    def bulk_upsert_embeddings(
        self,
        target: WeaviateTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        index_name = target.index_name
        field_name = target.field_name
        num_dimensions = len(embeddings[0])
        num_embeddings = len(embeddings)
        docs = []

        if not all(len(embedding) == num_dimensions for embedding in embeddings):
            raise ValueError("Not all embeddings have the same number of dimensions")

        if not len(ids) == num_embeddings:
            raise ValueError("Number of ids does not match number of embeddings")




        # Class definition object. Weaviate's autoschema feature will infer properties when importing.
        # We set vectorizer to none since we handle creating the embeddings ourselves
        class_obj = {
            "class": index_name,
            "vectorizer": "none",
        }


        if not self.wc.schema.exists(index_name):
            # Add the class to the schema
            # We never explicitly define a schema, Weaviate does this automatically
            self.wc.schema.create_class(class_obj)
        # else:
        #     print("Class already exists")



        # url = 'https://raw.githubusercontent.com/weaviate-tutorials/quickstart/main/data/jeopardy_tiny+vectors.json'
        # resp = requests.get(url)
        # data = json.loads(resp.text)

        # Configure a batch process
        with self.wc.batch as batch:
            batch.batch_size=100
            # Batch import all Questions
            for i, embedding in enumerate(embeddings):
                # print(f"importing question: {i+1}")


                self.wc.batch.add_data_object({}, index_name, vector=embedding)




        # pass

        # num_dimensions = len(embeddings[0])
        # num_embeddings = len(embeddings)

        # if not all(len(embedding) == num_dimensions for embedding in embeddings):
        #     raise ValueError("Not all embeddings have the same number of dimensions")

        # if not len(ids) == num_embeddings:
        #     raise ValueError("Number of ids does not match number of embeddings")

        # # # This will create the class if it doesn't exist, otherwise it will do nothing
        # # self._create_class(
        # #     class_name=index_name,
        # #     field_name=field_name,
        # #     num_dimensions=num_dimensions,
        # # )

        # vectors_data = []
        # for embedding in embeddings:
        #     vectors_data.append(
        #         {"class": index_name, "properties": {field_name: embedding}}
        #     )

        # self.wc.batch._create_data(vectors_data)
