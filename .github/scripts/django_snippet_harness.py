"""Execute extracted Django docs snippets against the local snippet models."""

# pylint: disable=import-error,too-few-public-methods

from __future__ import annotations

import os
from urllib.parse import urlparse

import django
from django.conf import settings
from django.contrib.postgres.fields import IntegerRangeField
from django.db import models

from paradedb.queryset import ParadeDBManager


def normalize_database_url(url: str) -> str:
    """Normalize legacy Postgres URLs for Django's database parser."""
    if url.startswith("postgres://"):
        return "postgresql://" + url[len("postgres://") :]
    return url


def database_settings() -> dict[str, object]:
    """Build Django database settings from `DATABASE_URL`."""
    parsed = urlparse(
        normalize_database_url(
            os.getenv(
                "DATABASE_URL", "postgresql://postgres:postgres@localhost:5432/postgres"
            )
        )
    )
    name = (parsed.path or "/postgres").lstrip("/") or "postgres"

    return {
        "ENGINE": "django.db.backends.postgresql",
        "NAME": name,
        "USER": parsed.username or "postgres",
        "PASSWORD": parsed.password or "",
        "HOST": parsed.hostname or "localhost",
        "PORT": int(parsed.port or 5432),
    }


def configure_django() -> None:
    """Initialize Django once for snippet execution."""
    if settings.configured:
        return

    settings.configure(
        DATABASES={"default": database_settings()},
        INSTALLED_APPS=[
            "django.contrib.contenttypes",
            "django.contrib.postgres",
        ],
        DEFAULT_AUTO_FIELD="django.db.models.BigAutoField",
        SECRET_KEY="paradedb-docs-snippets",
    )
    django.setup()


configure_django()


class MockItem(models.Model):
    """Read-only model used by docs snippets that target `mock_items`."""

    id = models.IntegerField(primary_key=True)
    description = models.TextField()
    rating = models.IntegerField()
    category = models.CharField(max_length=255)
    in_stock = models.BooleanField()
    metadata = models.JSONField(null=True)
    created_at = models.DateTimeField(null=True)
    last_updated_date = models.DateField(null=True)
    latest_available_time = models.TimeField(null=True)
    weight_range = IntegerRangeField(null=True)

    objects = ParadeDBManager()

    class Meta:
        """Bind the model to the existing docs verification table."""

        app_label = "docs_snippets"
        managed = False
        db_table = "mock_items"


class Order(models.Model):
    """Read-only model used by docs snippets that target `orders`."""

    order_id = models.IntegerField(primary_key=True)
    product = models.ForeignKey(
        MockItem,
        db_column="product_id",
        on_delete=models.DO_NOTHING,
        related_name="orders",
        to_field="id",
    )
    order_quantity = models.IntegerField()
    order_total = models.DecimalField(max_digits=10, decimal_places=2)
    customer_name = models.CharField(max_length=255)

    objects = ParadeDBManager()

    class Meta:
        """Bind the model to the existing docs verification table."""

        app_label = "docs_snippets"
        managed = False
        db_table = "orders"
