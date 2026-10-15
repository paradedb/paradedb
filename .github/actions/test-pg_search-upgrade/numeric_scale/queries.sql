-- The upgraded build routes numeric(3, 20) to NumericBytes, but this index stores the column as
-- `I64`. Without the compatibility arm in `derive_field_type_from_schema`, the NaN below is
-- written as bytes and every later query fails with
-- `Schema error: 'Expected a I64 for field "wide_scale"'`.
insert into items (id, wide_scale) values (2, 'NaN');

select id, wide_scale from items where id @@@ pdb.all() order by id;

select id, wide_scale from items where id @@@ pdb.all() and wide_scale = 'NaN'::numeric;
