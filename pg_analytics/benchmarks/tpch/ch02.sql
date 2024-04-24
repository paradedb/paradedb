with cheapest_part as
(
select
    min(ps_supplycost) as cp_lowest,
    p_partkey as cp_partkey
from part,
    partsupp,
    supplier,
    nation,
    region
where p_partkey = ps_partkey
    and s_suppkey = ps_suppkey
    and s_nationkey = n_nationkey
    and n_regionkey = r_regionkey
    and r_name = 'EUROPE'
group by p_partkey
)
select
	s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address,
	s_phone, s_comment
from part,
	supplier,
	partsupp,
	nation,
	region,
	cheapest_part
where p_partkey = ps_partkey
	and s_suppkey = ps_suppkey
	and p_size = 15
	and p_type like '%BRASS'
	and s_nationkey = n_nationkey
	and n_regionkey = r_regionkey
	and r_name = 'EUROPE'
	and ps_supplycost = cp_lowest
	and cp_partkey = p_partkey
order by s_acctbal desc,
	n_name,
	s_name,
	p_partkey
limit 10;
