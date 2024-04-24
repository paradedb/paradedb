select s_name, count(*) as numwait
from supplier, lineitem l1, orders, nation
where s_suppkey = l1.l_suppkey
	and o_orderkey = l1.l_orderkey
	and o_orderstatus = 'F'
	and l1.l_receiptdate > l1.l_commitdate
	and l1.l_orderkey in (
		select l_orderkey
		from lineitem
		group by l_orderkey
		having count(l_suppkey) > 1
	)
	and l1.l_orderkey not in (
		select l_orderkey
		from lineitem
		where l_receiptdate > l_commitdate
		group by l_orderkey
		having count(l_suppkey) > 1
	)
	and s_nationkey = n_nationkey
	and n_name = 'SAUDI ARABIA'
group by s_name
order by numwait desc, s_name
limit 100;
