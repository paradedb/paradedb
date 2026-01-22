\i common/common_setup.sql

DROP TABLE IF EXISTS "inventory_items";
CREATE TABLE "inventory_items" (
    "id" INT PRIMARY KEY,
    "supplier_id" INT,
    "condition" TEXT,
    "availability" TEXT,
    "customer_rating" FLOAT,
    "sales_rank" INT,
    "is_certified" BOOLEAN,
    "location_count" INT
);

SELECT setseed(0.42);

INSERT INTO "inventory_items" (
    "id", "supplier_id", "condition", "availability", "customer_rating", "sales_rank", "is_certified", "location_count"
)
SELECT
    g,
    CASE WHEN (random() < 0.1) THEN 115 ELSE (random() * 100)::int END,
    CASE WHEN (random() < 0.5) THEN 'new' WHEN (random() < 0.8) THEN 'used' ELSE 'refurbished' END,
    CASE WHEN (random() < 0.8) THEN 'available' ELSE 'out_of_stock' END,
    (random() * 100)::float,
    (random() * 100000)::int,
    (random() < 0.5),
    (floor(random() * 5) + 1)::int
FROM generate_series(1, 10000) AS g;

CREATE INDEX inventory_items_idx ON "inventory_items" USING bm25 (
    id, supplier_id, condition, availability, customer_rating, sales_rank, is_certified, location_count
) WITH (key_field='id');

SELECT "id"
FROM "inventory_items" AS "InventoryItems"
WHERE "InventoryItems"."id" @@@ '{"boolean":{"must":[{"term":{"field":"supplier_id","value":115}},{"boolean":{"should":[{"term":{"field":"condition","value":"new"}},{"term":{"field":"condition","value":"used"}}]}},{"term":{"field":"availability","value":"available"}}]}}'::jsonb
ORDER BY pdb.score("InventoryItems"."id") DESC
LIMIT 12
OFFSET 0;

DROP TABLE "inventory_items";
