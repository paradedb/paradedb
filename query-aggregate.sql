SET max_parallel_workers to 0;

SELECT *
FROM paradedb.aggregate(
    'contacts_companies_combined_full_idx',
    paradedb.boolean(
      must => ARRAY[
          paradedb.boolean(
              must => ARRAY[
                  paradedb.boolean(
                      should => ARRAY[paradedb.parse((
                          SELECT concat('contact_id:IN [', string_agg(ldf_id::TEXT, ' '), ']')
                          FROM contact_list
                          WHERE list_id IN ('tjy3slfS5wk')
                      ))]
                  )
              ]
          )
--           paradedb.boolean(
--               must => ARRAY[
--                   paradedb.boolean(
--                       must_not => ARRAY[
--                           paradedb.parse((
--                               SELECT concat('contact_id:IN [', string_agg(ldf_id::TEXT, ' '), ']')
--                               FROM contact_list
--                               WHERE list_id IN ('loxSXiPQRww','SZWEZLQhwbE')
--                       ))],
--                       must => ARRAY[paradedb.all()]
--                   )
--               ]
--           ),
--           paradedb.boolean(
--               should => ARRAY[
--                   paradedb.boolean(
--                       must => ARRAY[
--                           paradedb.boolean(must => ARRAY[paradedb.parse('contact_job_title:"Senior Programmer"')]),
--                           paradedb.boolean(must => ARRAY[paradedb.parse('contact_job_details.job_function:"product management, research, & innovation"')]),
--                           paradedb.boolean(must => ARRAY[paradedb.parse('contact_job_details.job_area:"software development & engineering"')])
--                       ])])
--           paradedb.boolean(must => ARRAY[paradedb.range(field => 'contact_id', range => '(0,)'::int8range)])
      ]
    ),
    '{
        "company_revenue_range_enum": {
            "terms": {
                "field": "contact_last_name",
                "size": 10,
                "segment_size": 10
            },
            "aggs": { "company_count": { "cardinality": { "field": "company_id" } } }
        }
    }',
    FALSE,
    1200000000
);
