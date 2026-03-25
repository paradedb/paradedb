from __future__ import annotations

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
from sqlalchemy.dialects.postgresql import INT4RANGE, JSONB
from sqlalchemy.engine import Engine
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


class Base(DeclarativeBase):
    pass


class MockItem(Base):
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
    __tablename__ = "orders"

    order_id: Mapped[int] = mapped_column(Integer, primary_key=True)
    product_id: Mapped[int] = mapped_column(ForeignKey("mock_items.id"), nullable=False)
    order_quantity: Mapped[int] = mapped_column(Integer, nullable=False)
    order_total: Mapped[Any] = mapped_column(Numeric(10, 2), nullable=False)
    customer_name: Mapped[str] = mapped_column(String(255), nullable=False)


def normalize_database_url(url: str) -> str:
    if url.startswith("postgres://"):
        url = "postgresql://" + url[len("postgres://") :]
    if url.startswith("postgresql://"):
        return "postgresql+psycopg://" + url[len("postgresql://") :]
    return url


def engine_from_env() -> Engine:
    dsn = os.getenv(
        "DATABASE_URL", "postgresql+psycopg://postgres:postgres@localhost:5432/postgres"
    )
    return create_engine(normalize_database_url(dsn), future=True)


engine = engine_from_env()

__all__ = [
    "Base",
    "MockItem",
    "Order",
    "engine",
    "engine_from_env",
    "normalize_database_url",
]
