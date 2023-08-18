/* eslint-disable @typescript-eslint/no-var-requires */
const { Client, Database, Table } = require("retake-search")
const dotenv = require("dotenv")

dotenv.config()

const setup = async () => {
  const client = new Client(
    process.env.RETAKE_API_KEY,
    process.env.RETAKE_API_URL
  )

  const database = new Database({
    host: process.env.DATABASE_HOST,
    user: process.env.DATABASE_USER,
    password: process.env.DATABASE_PASSWORD,
    port: parseInt(process.env.DATABASE_PORT || "5432"),
    dbName: process.env.DATABASE_NAME,
  })

  const table = new Table({
    table: process.env.DATABASE_TABLE_NAME,
    columns: JSON.parse(process.env.DATABASE_TABLE_COLUMNS || "[]"),
  })

  let index
  try {
    index = await client.getIndex(process.env.DATABASE_TABLE_NAME)
  } catch (err) {
    index = await client.createIndex(process.env.DATABASE_TABLE_NAME)
  }

  console.log("Vectorizing fields...")

  await index.vectorize({
    fieldNames: JSON.parse(process.env.DATABASE_TABLE_COLUMNS || "[]"),
  })
  await index.addSource(database, table)

  return
}

setup().then(() => console.log("Index created and source added"))
