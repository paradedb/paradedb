DROP TABLE IF EXISTS benchmark_logs CASCADE;

CREATE TABLE benchmark_logs (
    id SERIAL PRIMARY KEY,
    message TEXT,
    country VARCHAR(255),
    severity INTEGER,
    timestamp TIMESTAMP,
    metadata JSONB
);

\set num_rows :num_rows
\echo 'Generating' :num_rows 'rows'

INSERT INTO benchmark_logs (message, country, severity, timestamp, metadata)
SELECT
  (ARRAY[
    'The quick brown fox jumped over the lazy dog while the sun was setting behind the mountains, casting long shadows across the peaceful valley below.',
    'Scientists discovered a new species of deep-sea creature living near hydrothermal vents, marking a breakthrough in our understanding of extreme environment adaptations.',
    'The ancient library contained countless scrolls and manuscripts, each one holding secrets and knowledge from civilizations long forgotten by time.',
    'Through the telescope, astronomers observed a distant galaxy collision, its cosmic dance of stars and gas creating spectacular patterns across the night sky.',
    'The artificial intelligence system processed billions of calculations per second, working tirelessly to solve complex problems that had puzzled humans for centuries.',
    'Deep in the rainforest, rare flowers bloomed in brilliant colors, their sweet fragrance attracting exotic butterflies and hummingbirds from miles around.',
    'The quantum computer completed in microseconds what would have taken traditional computers millennia, ushering in a new era of computational capability.',
    'Archaeologists unearthed artifacts from an ancient civilization, each piece telling stories of daily life, customs, and beliefs from thousands of years ago.',
    'The space station crew conducted groundbreaking experiments in zero gravity, their research promising to revolutionize manufacturing and medicine on Earth.',
    'Through advanced genetic engineering, scientists developed crops that could thrive in harsh conditions, offering hope for food security in challenging climates.'
  ])[1 + MOD(s.id - 1, 10)],
  (ARRAY[
    'United States',
    'Canada',
    'United Kingdom',
    'France',
    'Germany',
    'Japan',
    'Australia',
    'Brazil',
    'India',
    'China'
  ])[1 + MOD(s.id - 1, 10)],
  1 + MOD(s.id - 1, 5),
  timestamp '2020-01-01' +
    make_interval(days => MOD(s.id - 1, 731)), -- 731 days = 2 years (including leap year)
  jsonb_build_object(
    'value', 1 + MOD(s.id - 1, 1000),
    'label', (ARRAY[
      'critical system alert',
      'routine maintenance',
      'security notification',
      'performance metric',
      'user activity',
      'system status',
      'network event',
      'application log',
      'database operation',
      'authentication event'
    ])[1 + MOD(s.id - 1, 10)]
  )
FROM generate_series(1, :num_rows) s(id);
