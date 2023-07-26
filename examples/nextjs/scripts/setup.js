const { Client } = require("retake-search");
require("dotenv").config();

const client = new Client({
  apiKey: process.env.RETAKE_API_KEY,
  url: process.env.RETAKE_API_URL,
});

console.log(
  "Indexing table (this could take a while if your table is large)..."
);

client
  .index({
    database: {
      host: process.env.DATABASE_HOST,
      port: process.env.DATABASE_PORT,
      user: process.env.DATABASE_USER,
      password: process.env.DATABASE_PASSWORD,
    },
    table: {
      name: process.env.DATABASE_TABLE_NAME,
      primaryKey: process.env.DATABASE_TABLE_PRIMARY_KEY,
      columns: JSON.parse(process.env.DATABASE_TABLE_COLUMNS),
    },
  })
  .then(console.log);
