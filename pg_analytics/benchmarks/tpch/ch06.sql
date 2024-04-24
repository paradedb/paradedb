select
	sum(l_extendedprice * l_discount) as revenue
from
	lineitem
where
	l_shipdate >= date '1994-01-01'
	and l_shipdate < date '1994-01-01' + interval '1' year
	and l_discount between toDecimal64(0.05,2) and toDecimal64(0.07,2)
	and l_quantity < 24;
