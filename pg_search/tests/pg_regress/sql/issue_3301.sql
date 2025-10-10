select count(*) from regress.mock_items where id @@@ pdb.all();
select count(*) from regress.mock_items where id @@@ pdb.empty();