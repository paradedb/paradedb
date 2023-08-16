FROM python:3.9-slim

WORKDIR /app

ARG COMMIT_SHA
ENV COMMIT_SHA=$COMMIT_SHA

# Install poetry
RUN apt-get update && apt-get install -y curl && \
    curl -sSL https://install.python-poetry.org | python -

# Configure poetry
ENV PATH="${PATH}:/root/.local/bin"

# Install dependencies
COPY pyproject.toml .
RUN poetry config virtualenvs.create false && poetry install

# Copy source
COPY . .

# Install Uvicorn for web service
RUN pip install uvicorn

# Expose port
EXPOSE 8000

# Run FastAPI server
CMD ["uvicorn", "api.app:app", "--host", "0.0.0.0", "--port", "8000"]
