SET
	session my.number_of_cities = '100';

SET
	session my.number_of_countries = '50';

CREATE EXTENSION IF NOT EXISTS pgcrypto;

INSERT INTO
	city
SELECT
	id,
	CONCAT('City ', id),
	CONCAT(
		'Country ',
		FLOOR(
			random() * (current_setting('my.number_of_countries') :: int) + 1
		) :: int
	)
FROM
	GENERATE_SERIES(1, current_setting('my.number_of_cities') :: int) AS id;