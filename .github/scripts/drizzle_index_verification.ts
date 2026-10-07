if (docsIndexes.length) {
  const { generateDrizzleJson, generateMigration } =
    await import("drizzle-kit/api-postgres");
  const table =
    process.env.DOCS_INDEX_TABLE === "array_demo" ? arrayDemo : mockItems;
  const before = await generateDrizzleJson({});
  const after = await generateDrizzleJson({ table });
  const statements = await generateMigration(before, after);
  const indexStatements = statements.filter((statement: string) =>
    /^CREATE (UNIQUE )?INDEX/i.test(statement),
  );
  if (indexStatements.length !== docsIndexes.length) {
    throw new Error(
      `Expected ${docsIndexes.length} index statements, got ${indexStatements.length}`,
    );
  }
  for (const statement of indexStatements) {
    if (!statement.includes("USING paradedb"))
      throw new Error(`Unexpected index DDL: ${statement}`);
    await client.unsafe(statement);
  }
  const indexes =
    await client`SELECT indexrelid FROM pg_index JOIN pg_class ON oid = indexrelid JOIN pg_am ON pg_am.oid = relam WHERE amname = 'paradedb' AND indrelid = ${table === arrayDemo ? "array_demo" : "mock_items"}::regclass AND indisvalid AND indisready`;
  if (indexes.length !== docsIndexes.length)
    throw new Error("Generated indexes are missing or invalid");
}
