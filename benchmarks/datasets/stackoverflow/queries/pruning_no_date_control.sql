-- Text-only control: the date partition column is absent from the query.

SELECT count(*) FROM stackoverflow_posts WHERE body ||| 'javascript';
