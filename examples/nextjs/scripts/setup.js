const { Client, Database, Table } = require("retake-search");
require("dotenv").config();

const setup = async () => {
  const client = new Client(
    process.env.RETAKE_API_KEY,
    process.env.RETAKE_API_URL
  );

  const database = new Database(
    process.env.DATABASE_HOST,
    process.env.DATABASE_USER,
    process.env.DATABASE_PASSWORD,
    parseInt(process.env.DATABASE_PORT || "5432"),
    process.env.DATABASE_NAME
  );

  const table = new Table(
    process.env.DATABASE_TABLE_NAME,
    process.env.DATABASE_TABLE_PRIMARY_KEY,
    JSON.parse(process.env.DATABASE_TABLE_COLUMNS || "[]")
  );

  console.log(
    "Indexing table (this could take a while if your table is large)..."
  );

  let index = await client.getIndex(process.env.DATABASE_TABLE_NAME);

  if (!index) {
    index = client.createIndex(process.env.DATABASE_TABLE_NAME);
  }

  if (!index) {
    throw new Error("Table failed to index due to an unexpected error");
  }

  console.log(
    "Vectorizing",
    JSON.parse(process.env.DATABASE_TABLE_COLUMNS || "[]")
  );

  await index.vectorize(JSON.parse(process.env.DATABASE_TABLE_COLUMNS || "[]"));
  await index.addSource(database, table);

  return;
};

setup().then(() => console.log("Index created and source added"));
