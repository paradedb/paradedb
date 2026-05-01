insert into items (id, numeric64, numeric_bytes) values (2, 0.098098098, 0.09809809809809809);

select id, numeric64, numeric_bytes from items where id @@@ pdb.all();
