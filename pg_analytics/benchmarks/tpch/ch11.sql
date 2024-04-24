select
	ps_partkey,
	sum(ps_supplycost * ps_availqty) as value
from partsupp,
	supplier,
	nation
where ps_suppkey = s_suppkey
	and s_nationkey = n_nationkey
	and n_name = 'GERMANY'
group by ps_partkey 
having sum(ps_supplycost * ps_availqty) >
		( select toDecimal64(sum(ps_supplycost * ps_availqty) * 0.0000001,10)
			-- The above constant needs to be adjusted according
			-- to the scale factor (SF): constant = 0.0001 / SF.
		from partsupp,
			supplier,
			nation
		where ps_suppkey = s_suppkey
			and s_nationkey = n_nationkey
			and n_name = 'GERMANY'
	)
order by value desc;
