DROP TABLE IF EXISTS benchmark_logs CASCADE;

CREATE TABLE benchmark_logs (
    id SERIAL PRIMARY KEY,
    message TEXT,
    country VARCHAR(255),
    severity INTEGER,
    timestamp TIMESTAMP,
    metadata JSONB
);

\set rows :rows
\echo 'Generating' :rows 'rows'

INSERT INTO benchmark_logs (message, country, severity, timestamp, metadata)
SELECT
  (ARRAY[
    'The research team discovered a new species of deep-sea creature while conducting experiments near hydrothermal vents in the dark ocean depths.',
    'The research facility analyzed samples from ancient artifacts, revealing breakthrough findings about civilizations lost to the depths of time.',
    'The research station monitored weather patterns across mountain peaks, collecting data about atmospheric changes in the remote depths below.',
    'The research observatory captured images of stellar phenomena, peering into the cosmic depths to understand the mysteries of distant galaxies.',
    'The research laboratory processed vast amounts of genetic data, exploring the molecular depths of DNA to unlock biological secrets.',
    'The research center studied rare organisms found in ocean depths, documenting new species thriving in extreme underwater environments.',
    'The research institute developed quantum systems to probe subatomic depths, advancing our understanding of fundamental particle physics.',
    'The research expedition explored underwater depths near volcanic vents, discovering unique ecosystems adapted to extreme conditions.',
    'The research facility conducted experiments in the depths of space, testing how different materials behave in zero gravity environments.',
    'The research team engineered crops that could grow in the depths of drought conditions, helping communities facing climate challenges.'
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
FROM generate_series(1, :rows) s(id);
