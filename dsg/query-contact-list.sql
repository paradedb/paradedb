SELECT * FROM contacts_companies_combined_full WHERE contact_id @@@ paradedb.term_set(
    'contact_id',
    (
        SELECT ARRAY(
            SELECT ldf_id
            FROM contact_list
            WHERE list_id IN ('21430')
        ) a
    )
);
