FROM python:3.11-slim

WORKDIR /usr/src/app

RUN pip install --no-cache-dir confluent-kafka==2.2.0 requests==2.31.0

COPY . .

CMD [ "python", "./consumer.py" ]
