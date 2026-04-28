"""Execute extracted Django docs snippets against the local snippet models."""

# pylint: disable=import-error,too-few-public-methods

from __future__ import annotations

import getpass
import os

import django
from django.conf import settings
from django.contrib.postgres.fields import ArrayField, IntegerRangeField
from django.db import models

from paradedb.queryset import ParadeDBManager

DATABASES = {
    "default": {
        "ENGINE": "django.db.backends.postgresql",
        "NAME": os.environ.get("PARADEDB_DATABASE", "postgres"),
        "USER": os.environ.get("PARADEDB_USER", getpass.getuser()),
        "PASSWORD": os.environ.get("PARADEDB_PASSWORD", ""),
        "HOST": os.environ.get("PARADEDB_HOST", "localhost"),
        "PORT": int(os.environ.get("PARADEDB_PORT", "28818")),
    }
}


def configure_django() -> None:
    """Initialize Django once for snippet execution."""
    if settings.configured:
        return

    settings.configure(
        DATABASES=DATABASES,
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


class ArrayDemo(models.Model):
    """Read-only model used by docs snippets that target `array_demo`."""

    id = models.IntegerField(primary_key=True)
    categories = ArrayField(models.TextField())

    class Meta:
        """Bind the model to the existing docs verification table."""

        app_label = "docs_snippets"
        managed = False
        db_table = "array_demo"
