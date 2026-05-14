import { drizzle as drizzlePostgres } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import {
  customType as pgCustomType,
  integer as pgInteger,
  pgTable as definePgTable,
  text as pgText,
  varchar as pgVarchar,
} from "drizzle-orm/pg-core";

const connectionString =
  process.env.DATABASE_URL ??
  "postgres://postgres:postgres@localhost:5432/postgres";

const client = postgres(connectionString);
const db = drizzlePostgres({ client });

const int4range = pgCustomType<{ data: string; driverData: string }>({
  dataType() {
    return "int4range";
  },
});

const mockItems = definePgTable("mock_items", {
  id: pgInteger("id").primaryKey(),
  description: pgText("description"),
  rating: pgInteger("rating"),
  category: pgVarchar("category", { length: 255 }),
  weightRange: int4range("weight_range"),
});

const orders = definePgTable("orders", {
  orderId: pgInteger("order_id").primaryKey(),
  productId: pgInteger("product_id").notNull(),
  customerName: pgVarchar("customer_name", { length: 255 }).notNull(),
});
