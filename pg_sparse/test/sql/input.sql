SELECT '[1,2,3]'::svector;
SELECT '[-1,-2,-3]'::svector;
SELECT '[1.,2.,3.]'::svector;
SELECT ' [ 1,  2 ,    3  ] '::svector;
SELECT '[1.23456]'::svector;
SELECT '[hello,1]'::svector;
SELECT '[NaN,1]'::svector;
SELECT '[Infinity,1]'::svector;
SELECT '[-Infinity,1]'::svector;
SELECT '[1.5e38,-1.5e38]'::svector;
SELECT '[1.5e+38,-1.5e+38]'::svector;
SELECT '[1.5e-38,-1.5e-38]'::svector;
SELECT '[4e38,1]'::svector;
SELECT '[1,2,3'::svector;
SELECT '[1,2,3]9'::svector;
SELECT '1,2,3'::svector;
SELECT ''::svector;
SELECT '['::svector;
SELECT '[,'::svector;
SELECT '[]'::svector;
SELECT '[1,]'::svector;
SELECT '[1a]'::svector;
SELECT '[1,,3]'::svector;
SELECT '[1, ,3]'::svector;
SELECT '[1,2,3]'::svector(2);

SELECT unnest('{"[1,2,3]", "[4,5,6]"}'::svector[]);
SELECT '{"[1,2,3]"}'::svector(2)[];
