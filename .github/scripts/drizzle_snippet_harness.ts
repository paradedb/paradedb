import { drizzle as drizzlePostgres } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import {
  boolean as pgBoolean,
  customType as pgCustomType,
  jsonb as pgJsonb,
  integer as pgInteger,
  pgTable as definePgTable,
  serial as pgSerial,
  text as pgText,
  timestamp as pgTimestamp,
  varchar as pgVarchar,
} from "drizzle-orm/pg-core";

const connectionString =
  process.env.DATABASE_URL ??
  "postgres://postgres:postgres@localhost:5432/postgres";

const client = postgres(connectionString);
const db = drizzlePostgres({ client });

const int4range = pgCustomType({
  dataType() {
    return "int4range";
  },
});

const mockItems = definePgTable("mock_items", {
  id: pgInteger("id").primaryKey(),
  description: pgText("description"),
  rating: pgInteger("rating"),
  category: pgVarchar("category", { length: 255 }),
  inStock: pgBoolean("in_stock"),
  createdAt: pgTimestamp("created_at"),
  metadata: pgJsonb("metadata"),
  weightRange: int4range("weight_range"),
});

const orders = definePgTable("orders", {
  orderId: pgInteger("order_id").primaryKey(),
  productId: pgInteger("product_id").notNull(),
  customerName: pgVarchar("customer_name", { length: 255 }).notNull(),
});

const arrayDemo = definePgTable("array_demo", {
  id: pgSerial("id").primaryKey(),
  categories: pgText("categories").array(),
});
