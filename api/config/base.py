import os


class Config:
    def get_property(self, property_name: str) -> str:
        value = os.environ.get(property_name)
        if not value:
            raise EnvironmentError(
                f"{property_name} environment variable is not defined."
            )
        return value
