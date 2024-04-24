with filt_orders as (
    select * from orders
    where o_comment not like '%special%requests%'
    )
select c_count,
count(*) as custdist
from (
    select c_custkey,
        count(o_orderkey) as c_count
    from customer left outer join filt_orders on
        c_custkey = o_custkey 
    group by c_custkey
) as c_orders 
group by c_count
order by custdist desc, c_count desc;
