CREATE TABLE employees (salary bigint, id smallint) USING deltalake;
INSERT INTO employees VALUES (100, 1), (200, 2), (300, 3), (400, 4), (500, 5);
DELETE FROM employees WHERE id = 5 OR salary <= 200;
SELECT * FROM employees;

DELETE FROM employees;

CREATE TABLE projects (project_id serial, employee_id int) using deltalake;
DELETE FROM employees
WHERE id NOT IN (SELECT employee_id FROM projects);

DROP TABLE employees, projects;
