DO $$
BEGIN
    IF (SELECT count(*) FROM legacy_inet_items WHERE ip = '10.0.0.1'::inet) <> 1 THEN
        RAISE EXCEPTION 'normal inet comparison failed';
    END IF;
END;
$$;

DO $$
BEGIN
    IF (SELECT count(*) FROM legacy_inet_items WHERE ip @@@ pdb.term('10.0.0.1'::inet)) <> 1 THEN
        RAISE EXCEPTION 'pdb.term on legacy inet index failed';
    END IF;
END;
$$;

insert into legacy_inet_items values
    (3, '10.0.0.3', ARRAY['192.168.1.1', '192.168.1.2']::inet[]);

DO $$
BEGIN
    IF (SELECT count(*) FROM legacy_inet_items WHERE ips @@@ pdb.term('192.168.1.1'::inet)) <> 1 THEN
        RAISE EXCEPTION 'insert into legacy inet array index failed';
    END IF;
END;
$$;

delete from legacy_inet_items where id = 3;

DO $$
BEGIN
    IF (SELECT count(*) FROM legacy_inet_items WHERE ips @@@ pdb.term('192.168.1.1'::inet)) <> 0 THEN
        RAISE EXCEPTION 'delete from legacy inet array index failed';
    END IF;
END;
$$;
