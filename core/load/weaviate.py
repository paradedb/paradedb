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


    # In Weaviate, an index is called an object, which must be part of
    # a class, which is itself part of a schema. Weaviate automatically
    # generates a default schema if nothing is specified, and to keep things
    # simple we create a class for each object/index to keep 1:1 mapping.


    def _check_class_exists(self, index_name: str) -> bool:
        try:
            self.wc.schema.exists(index_name)
            return True
        except weaviate.exceptions.WeaviateBaseError as e:
            return False
        
    # In Weaviate, an index is called an object
    def _check_index_exists(self, index_name: str) -> bool:
        # Objects are referenced via UUID, which is a hash of the class name
        index_uuid = weaviate.util.generate_uuid5(index_name)
        try:
            self.wc.data_object.exists(index_uuid, class_name=index_name)
            return True
        except weaviate.exceptions.ObjectAlreadyExistsException as e:
                return False


             




    def _get_num_dimensions(self, index_name: str) -> int:
        
        
        print(self.wc.data_object.get(
            uuid = weaviate.util.generate_uuid5(index_name),
            with_vector = True,
            class_name = index_name
        ))
            
            
            
                    
        return int(len(self.wc.data_object.get(
            class_name=index_name,
        )[0]))
        
        


    def _create_class(self, index_name: str, num_dimensions: int) -> None:
        self.we.schema.create_class({
            "class": index_name,
            "vectorizer": "none",
        })

    def _create_index(self, index_name: str, num_dimensions: int) -> None:
        # Objects are referenced via UUID, which is a hash of the class name
        index_uuid = weaviate.util.generate_uuid5(index_name)
        self.wc.data_object.create(
             data_object = {},
             class_name = index_name,
             uuid = index_uuid,
             vector = [None] * num_dimensions # Initialize empty vector, we will fill it at upsert time
        )




    




    def check_and_setup_index(
        self, target: WeaviateTarget, num_dimensions: int 
    ) -> None:
        index_name = target.index_name

        if not self._check_class_exists(index_name=index_name):
            self._create_class(index_name=index_name)
        else:
            # Class already exists, check if object exists within it
            if not self._check_index_exists(index_name=index_name):
                self._create_index(index_name=index_name)
            else:
                # TODO: fix bug here
                # Weaviate does not appear to pre-create fixed dimensions indices, and this
                # is the limit on their dimensions: https://weaviate.io/developers/weaviate/more-resources/faq#what-is-the-maximum-number-of-vector-dimensions-for-embeddings
                index_dimensions = self._get_num_dimensions(index_name=index_name)
                if index_dimensions != num_dimensions:
                    raise ValueError(
                        f"Index {index_name} already exists with {index_dimensions} dimensions but embedding has {num_dimensions}"
                    )

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
