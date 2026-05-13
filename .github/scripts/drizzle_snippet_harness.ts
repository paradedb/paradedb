import {
  boolean,
  customType,
  date,
  integer,
  jsonb,
  pgTable,
  text,
  time,
  timestamp,
  varchar,
} from "drizzle-orm/pg-core";

const int4range = customType<{ data: string; driverData: string }>({
  dataType() {
    return "int4range";
  },
});

const mockItems = pgTable("mock_items", {
  id: integer("id").primaryKey(),
  description: text("description"),
  rating: integer("rating"),
  category: varchar("category", { length: 255 }),
  inStock: boolean("in_stock"),
  metadata: jsonb("metadata"),
  createdAt: timestamp("created_at"),
  lastUpdatedDate: date("last_updated_date"),
  latestAvailableTime: time("latest_available_time"),
  weightRange: int4range("weight_range"),
});
