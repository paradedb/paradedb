SELECT * FROM contacts_companies_combined_full
WHERE company_id IN (
    SELECT ldf_id
    FROM company_list
    WHERE list_id IN ('2543')
);