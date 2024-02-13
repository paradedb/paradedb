CREATE TABLE t (a int, b int);
INSERT INTO t VALUES (1, 2);
CREATE TABLE s (a int, b int) USING parquet;
INSERT INTO s SELECT * FROM t;
SELECT * FROM s;
DROP TABLE s, t;

CREATE TABLE t (
    id SERIAL PRIMARY KEY,
    event_date DATE,
    user_id INT,
    event_name VARCHAR(50),
    session_duration INT,
    page_views INT,
    revenue DECIMAL(10, 2)
) USING parquet;
INSERT INTO t (event_date, user_id, event_name, session_duration, page_views, revenue)
VALUES
(NULL, NULL, NULL, NULL, NULL, NULL);
SELECT * FROM t;
DROP TABLE t;
