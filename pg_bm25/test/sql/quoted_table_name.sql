CREATE TABLE "Activity" (name TEXT, age INTEGER);
INSERT INTO "Activity" (name, age) VALUES ('Alice', 29);
INSERT INTO "Activity" (name, age) VALUES ('Bob', 34);
INSERT INTO "Activity" (name, age) VALUES ('Charlie', 45);
INSERT INTO "Activity" (name, age) VALUES ('Diana', 27);
INSERT INTO "Activity" (name, age) VALUES ('Fiona', 38);
INSERT INTO "Activity" (name, age) VALUES ('George', 41);
INSERT INTO "Activity" (name, age) VALUES ('Hannah', 22);
INSERT INTO "Activity" (name, age) VALUES ('Ivan', 30);
INSERT INTO "Activity" (name, age) VALUES ('Julia', 25);
CREATE INDEX ON "Activity" USING bm25(("Activity".*)) WITH (text_fields='{"name": {}}');
SELECT * FROM "Activity" WHERE "Activity" @@@ 'alice';

