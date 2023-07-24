import os
from typing import Optional
from dotenv import dotenv_values

env_file = "retake.env"
config = dotenv_values(".env")


class SecretNotFoundError(Exception):
    pass


class SecretInvalidFormatError(Exception):
    pass


def store_env_secret(key: str, value: str) -> None:
    with open("retake.env", "r") as f:
        for line in f.readlines():
            try:
                key, value = line.split("=")
                os.putenv(key, value)
            except ValueError:
                raise SecretInvalidFormatError(
                    "Invalid secret. Make sure the secret is in KEY=value format"
                )


def get_env_secret(key: str) -> Optional[str]:
    if key in config:
        return config[key]
    else:
        raise SecretNotFoundError(f"Secret with key '{key}' not found")
