CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS authors;
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS reviews;

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    metadata JSONB
);

CREATE TABLE books (
    id SERIAL PRIMARY KEY,
    author_id INT,
    metadata JSON
);

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    book_id INT,
    metadata JSONB
);

INSERT INTO authors (name, metadata) VALUES
('J.K. Rowling', '{"age": 55, "text": "British author best known for the Harry Potter fantasy series"}'),
('Stephen King', '{"age": 75, "text": "American author known for his horror and supernatural fiction novels"}'),
('Agatha Christie', '{"age": 80, "text": "English writer known for her detective novels featuring Hercule Poirot"}'),
('Dan Brown', '{"age": 60, "text": "American author of thriller novels including The Da Vinci Code"}'),
('J.R.R. Tolkien', '{"age": 100, "text": "English author and philologist famous for The Lord of the Rings"}');

INSERT INTO books (author_id, metadata) VALUES
(2, '{"content": "This is a test test of the snippet function with multiple test words", "titles": ["test", "snippet", "function"], "test": "test"}'),
(2, '{"content": "Another test of the snippet snippet function with repeated snippet words", "titles": ["test", "test", "function"], "test": "test"}'),
(1, '{"content": "Yet another test test test of the function function function", "titles": ["test", "snippet", "test"], "test": "test"}'),
(3, '{"content": "test Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur, vel illum qui dolorem eum fugiat quo voluptas nulla pariatur? test At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus. Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet ut et voluptates repudiandae sint et molestiae non recusandae. Itaque earum rerum hic tenetur a sapiente delectus, ut aut reiciendis voluptatibus maiores alias consequatur aut perferendis doloribus asperiores repellat. test", "titles": ["test", "snippet", "function"], "test": "test"}');

INSERT INTO reviews (book_id, metadata) VALUES
(1, '{"review": "This is a test review of the snippet function with multiple test words"}'),
(2, '{"review": "Another test review of the snippet snippet function with repeated snippet words"}'),
(3, '{"review": "Yet another test review of the function function function"}'),
(3, '{"review": "test review of the snippet function with multiple test words"}'),
(2, '{"review": "test review of the snippet snippet function with repeated snippet words"}'),
(1, '{"review": "test review of the function function function"}');

CREATE INDEX ON authors USING bm25 (
    id,
    name,
    metadata
) WITH (key_field = 'id');

CREATE INDEX ON books USING bm25 (
    id,
    author_id,
    metadata
) WITH (key_field = 'id');

CREATE INDEX ON reviews USING bm25 (
    id,
    book_id,
    metadata
) WITH (key_field = 'id');
