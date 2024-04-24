with part_avg as (
	    select toDecimal64(0.2 * avg(l_quantity),12) as limit_qty, l_partkey as lpk
	    from lineitem
	    group by l_partkey
)
select sum(l_extendedprice) / toDecimal64(7.0,2) as avg_yearly
from lineitem, part, part_avg
where p_partkey = l_partkey
    and p_brand = 'Brand#23'
    and p_container = 'MED BOX'
    and p_partkey = lpk
    and l_quantity < limit_qty;
