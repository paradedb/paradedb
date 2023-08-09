FROM python:3.9-slim

WORKDIR /app

COPY ./sync/requirements.txt .

RUN pip install -r requirements.txt

COPY ./sync .

EXPOSE 7433

CMD ["uvicorn", "sidecar:app", "--host", "0.0.0.0", "--port", "7433"]
