-- SELECT '[1,2,3]'::svector + '[4,5,6]';
-- SELECT '[3e38]'::svector + '[3e38]';
-- SELECT '[1,2,3]'::svector - '[4,5,6]';
-- SELECT '[-3e38]'::svector - '[3e38]';
-- SELECT '[1,2,3]'::svector * '[4,5,6]';
-- SELECT '[1e37]'::svector * '[1e37]';
-- SELECT '[1e-37]'::svector * '[1e-37]';

-- SELECT svector_dims('[1,2,3]');

-- SELECT round(svector_norm('[1,1]')::numeric, 5);
-- SELECT svector_norm('[3,4]');
-- SELECT svector_norm('[0,1]');
-- SELECT svector_norm('[3e37,4e37]')::real;

-- SELECT l2_distance('[0,0]', '[3,4]');
-- SELECT l2_distance('[0,0]', '[0,1]');
-- SELECT l2_distance('[1,2]', '[3]');
-- SELECT l2_distance('[3e38]', '[-3e38]');

-- SELECT inner_product('[1,2]', '[3,4]');
-- SELECT inner_product('[1,2]', '[3]');
-- SELECT inner_product('[3e38]', '[3e38]');

-- SELECT cosine_distance('[1,2]', '[2,4]');
-- SELECT cosine_distance('[1,2]', '[0,0]');
-- SELECT cosine_distance('[1,1]', '[1,1]');
-- SELECT cosine_distance('[1,0]', '[0,2]');
-- SELECT cosine_distance('[1,1]', '[-1,-1]');
-- SELECT cosine_distance('[1,2]', '[3]');
-- SELECT cosine_distance('[1,1]', '[1.1,1.1]');
-- SELECT cosine_distance('[1,1]', '[-1.1,-1.1]');
-- SELECT cosine_distance('[3e38]', '[3e38]');

-- SELECT l1_distance('[0,0]', '[3,4]');
-- SELECT l1_distance('[0,0]', '[0,1]');
-- SELECT l1_distance('[1,2]', '[3]');
-- SELECT l1_distance('[3e38]', '[-3e38]');

-- SELECT sum(v) FROM unnest(ARRAY['[1,2,3]'::svector, '[3,5,7]']) v;
-- SELECT sum(v) FROM unnest(ARRAY['[1,2,3]'::svector, '[3,5,7]', NULL]) v;
-- SELECT sum(v) FROM unnest(ARRAY[]::svector[]) v;
-- SELECT sum(v) FROM unnest(ARRAY['[1,2]'::svector, '[3]']) v;
-- SELECT sum(v) FROM unnest(ARRAY['[3e38]'::svector, '[3e38]']) v;
