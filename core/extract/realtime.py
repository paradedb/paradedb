from http import HTTPStatus
import requests
import time
from loguru import logger

from typing import Any


def create_connector(connect_server: str, connector_config: dict[str, Any]) -> None:
    max_retries = 15
    retry_count = 0
    while retry_count < max_retries:
        try:
            time.sleep(1)  # Wait for 1 second before retrying
            url = f"{connect_server}/connectors"
            r = requests.post(
                url,
                json=connector_config,
            )
            if r.status_code == HTTPStatus.OK or r.status_code == HTTPStatus.CREATED:
                logger.info("Connector successfully created")
                break
            elif r.status_code == HTTPStatus.CONFLICT:
                raise requests.exceptions.HTTPError(HTTPStatus.CONFLICT)
                break
            else:
                logger.info(r.json())
                raise requests.exceptions.RequestException(
                    f"Failed to create connector: {r.reason}"
                )
        except requests.exceptions.ConnectionError:
            logger.info("Kafka connect server is not yet available, retrying...")
            retry_count += 1
            continue
