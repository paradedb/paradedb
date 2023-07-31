FROM python:3.9-slim

WORKDIR /app

RUN pip install retake-pgsync==2.5.1 starlette==0.31.0 loguru==0.7.0 uvicorn

COPY . .

EXPOSE 7433

CMD ["uvicorn", "sidecar:app", "--host", "0.0.0.0", "--port", "7433", "--reload"]

