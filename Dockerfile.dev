FROM python:3.9

WORKDIR /app

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
