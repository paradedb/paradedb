SELECT * FROM contacts_companies_combined_full WHERE contact_id @@@ paradedb.term_set(
    'company_id',
    (
        SELECT ARRAY(
            SELECT ldf_id
            FROM company_list
            WHERE list_id IN ('2543')
        ) a
    )
);
