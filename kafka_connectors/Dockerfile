FROM python:3.11-slim

WORKDIR /usr/src

RUN pip install --no-cache-dir confluent-kafka==2.2.0 requests==2.31.0

ENV PYTHONPATH "${PYTHONPATH}:./kafka_connectors:./core"

COPY ../core/ ./core
COPY . ./kafka_connectors

CMD [ "python", "-u", "kafka_connectors/controller.py" ]
