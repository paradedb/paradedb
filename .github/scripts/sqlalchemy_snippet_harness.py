"""Execute extracted SQLAlchemy docs snippets against local verification tables."""

# pylint: disable=import-error,too-few-public-methods

from __future__ import annotations

import getpass
import os
from typing import Any

from sqlalchemy import (
    Boolean,
    Date,
    DateTime,
    ForeignKey,
    Integer,
    Numeric,
    String,
    Text,
    Time,
    create_engine,
)
from sqlalchemy.dialects.postgresql import ARRAY, INT4RANGE, JSONB
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


def build_database_url() -> str:
    """Build a SQLAlchemy connection string for the docs verification database."""
    user = os.environ.get("PARADEDB_USER", getpass.getuser())
    password = os.environ.get("PARADEDB_PASSWORD", "")
    host = os.environ.get("PARADEDB_HOST", "localhost")
    port = os.environ.get("PARADEDB_PORT", "28818")
    database = os.environ.get("PARADEDB_DATABASE", "postgres")

    credentials = user
    if password:
        credentials = f"{credentials}:{password}"

    return f"postgresql+psycopg://{credentials}@{host}:{port}/{database}"


DATABASE_URL = build_database_url()


class Base(DeclarativeBase):
    """Declarative base shared by snippet verification models."""


class MockItem(Base):
    """Read-only model used by docs snippets that target `mock_items`."""

    __tablename__ = "mock_items"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    description: Mapped[str] = mapped_column(Text, nullable=False)
    rating: Mapped[int] = mapped_column(Integer, nullable=False)
    category: Mapped[str] = mapped_column(String(255), nullable=False)
    in_stock: Mapped[bool] = mapped_column(Boolean, nullable=False)
    metadata_: Mapped[Any] = mapped_column("metadata", JSONB, nullable=True)
    created_at: Mapped[Any] = mapped_column(DateTime, nullable=True)
    last_updated_date: Mapped[Any] = mapped_column(Date, nullable=True)
    latest_available_time: Mapped[Any] = mapped_column(Time, nullable=True)
    weight_range: Mapped[Any] = mapped_column(INT4RANGE, nullable=True)


class Order(Base):
    """Read-only model used by docs snippets that target `orders`."""

    __tablename__ = "orders"

    order_id: Mapped[int] = mapped_column(Integer, primary_key=True)
    product_id: Mapped[int] = mapped_column(ForeignKey("mock_items.id"), nullable=False)
    order_quantity: Mapped[int] = mapped_column(Integer, nullable=False)
    order_total: Mapped[Any] = mapped_column(Numeric(10, 2), nullable=False)
    customer_name: Mapped[str] = mapped_column(String(255), nullable=False)


class ArrayDemo(Base):
    """Read-only model used by docs snippets that target `array_demo`."""

    __tablename__ = "array_demo"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    categories: Mapped[list[str]] = mapped_column(ARRAY(Text), nullable=False)


engine = create_engine(DATABASE_URL, future=True)

__all__ = [
    "Base",
    "DATABASE_URL",
    "ArrayDemo",
    "MockItem",
    "Order",
    "engine",
]
