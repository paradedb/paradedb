import openai


class OpenAI:
    def __init__(self, api_key: str, model: str):
        openai.api_key = api_key
        self.model = model

    def create_embedding(self, document: str):
        return openai.Embedding.create(input=[document], model=self.model)
