CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

-- Setup test tables
drop table if exists orders_part cascade;
drop table if exists items cascade;
drop table if exists orders cascade;

call paradedb.create_bm25_test_table(schema_name => 'public', table_name => 'items', table_type => 'Items');
call paradedb.create_bm25_test_table(schema_name => 'public', table_name => 'orders', table_type => 'Orders');

-- Create item index
create index items_idx on items using bm25 (id, description) with (key_field = 'id');

-- Setup partitioned orders table
create table orders_part (
    like orders,
    part_key int not null,
    primary key (order_id, part_key)
) partition by list (part_key);

create table orders_part_0 partition of orders_part for values in (0);
create table orders_part_1 partition of orders_part for values in (1);

-- Insert data
insert into orders_part select *, order_id % 2 from orders;

-- Create partition index
create index orders_part_idx on orders_part
using bm25 (order_id, part_key, product_id, order_total, customer_name) with (key_field = 'order_id');

-- Test the join across the partition
select o.order_id, o.customer_name, i.description 
from orders_part o 
join items i on o.product_id = i.id 
where i.description ||| 'keyboard' or o.customer_name ||| 'John' 
order by o.order_total desc, o.order_id desc limit 5;

-- Direct query 
select * from orders_part where customer_name ||| 'John';

-- Edge case: Create and query an empty partition
create table orders_part_empty partition of orders_part for values in (999);
select * from orders_part where customer_name ||| 'Nobody';

-- Cleanup
drop table orders_part cascade;
drop table items cascade;
drop table orders cascade;
