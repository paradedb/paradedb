"""Execute extracted Django docs snippets against the local snippet models."""

# pylint: disable=import-error,too-few-public-methods

from __future__ import annotations

import ast
import os
from collections.abc import Mapping, Sequence
from urllib.parse import urlparse

import django
from django.conf import settings
from django.contrib.postgres.fields import IntegerRangeField
from django.db import models
from django.db.models import QuerySet

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


def execute(value: object) -> object:
    """Materialize queryset-like results nested inside common containers."""
    if isinstance(value, QuerySet):
        list(value)
        return value

    if isinstance(value, Mapping):
        for item in value.values():
            execute(item)
        return value

    if isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        for item in value:
            execute(item)
        return value

    return value


def _capture_last_statement(module: ast.Module) -> None:
    """Assign the snippet's final expression or named assignment to a result."""
    result_name = "__snippet_result__"

    if not module.body:
        module.body.append(
            ast.Assign(
                targets=[ast.Name(id=result_name, ctx=ast.Store())],
                value=ast.Constant(None),
            )
        )
        return

    last_statement = module.body[-1]

    if isinstance(last_statement, ast.Expr):
        module.body[-1] = ast.Assign(
            targets=[ast.Name(id=result_name, ctx=ast.Store())],
            value=last_statement.value,
        )
        return

    if isinstance(last_statement, ast.Assign) and len(last_statement.targets) == 1:
        target = last_statement.targets[0]
        if isinstance(target, ast.Name):
            module.body.append(
                ast.Assign(
                    targets=[ast.Name(id=result_name, ctx=ast.Store())],
                    value=ast.Name(id=target.id, ctx=ast.Load()),
                )
            )
            return

    if isinstance(last_statement, ast.AnnAssign) and isinstance(
        last_statement.target, ast.Name
    ):
        module.body.append(
            ast.Assign(
                targets=[ast.Name(id=result_name, ctx=ast.Store())],
                value=ast.Name(id=last_statement.target.id, ctx=ast.Load()),
            )
        )
        return

    module.body.append(
        ast.Assign(
            targets=[ast.Name(id=result_name, ctx=ast.Store())],
            value=ast.Constant(None),
        )
    )


def execute_snippet(source: str, *, filename: str = "<django-snippet>") -> object:
    """Execute a snippet and return its final value after materialization."""
    module = ast.parse(source, filename=filename, mode="exec")
    _capture_last_statement(module)
    ast.fix_missing_locations(module)

    namespace: dict[str, object] = {
        "MockItem": MockItem,
        "Order": Order,
    }
    exec(compile(module, filename, "exec"), namespace, namespace)  # pylint: disable=exec-used
    return execute(namespace["__snippet_result__"])


__all__ = [
    "MockItem",
    "Order",
    "configure_django",
    "database_settings",
    "execute",
    "execute_snippet",
    "normalize_database_url",
]
