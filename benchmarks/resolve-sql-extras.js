// Resolve content-addressed SQL metadata without discarding historical query variants.
async function resolveSqlExtras(data) {
    const references = Object.values(data?.entries || {}).flat()
        .flatMap(run => run.benches || [])
        .filter(bench => bench.extra?.includes('sql-extra:sha256:'));
    if (!references.length) return;
    const response = await fetch('sql-extras.json');
    if (!response.ok) throw new Error(`Cannot load SQL metadata: ${response.status}`);
    const extras = await response.json();
    for (const bench of references) {
        const [prefix, key] = bench.extra.split('sql-extra:sha256:');
        if (!(key in extras)) throw new Error(`Missing SQL metadata: ${key}`);
        bench.extra = prefix + extras[key];
    }
}
