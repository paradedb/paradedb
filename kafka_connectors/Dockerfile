FROM python:3.11-slim

WORKDIR /usr/src

COPY kafka_connectors/requirements.txt .
RUN pip install -r requirements.txt

ENV PYTHONPATH "${PYTHONPATH}:./kafka_connectors:./core"

COPY ./core/ ./core
COPY ./kafka_connectors ./kafka_connectors

CMD [ "python", "-u", "kafka_connectors/controller.py" ]
