FROM python:3.11-slim

WORKDIR /usr/src

RUN pip install --no-cache-dir confluent-kafka==2.2.0 requests==2.31.0

ENV PYTHONPATH "${PYTHONPATH}:./connectors"

COPY . ./connectors

CMD [ "python", "-u", "connectors/consumer.py" ]
