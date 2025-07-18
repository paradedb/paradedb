CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS authors;
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS reviews;

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    age INT
);

CREATE TABLE books (
    id INT,
    author_id INT,
    content TEXT,
    titles TEXT[],
    metadata JSONB,
    PRIMARY KEY (id, author_id)
);

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    book_id INT,
    review TEXT
);

INSERT INTO authors (name, age) VALUES
('J.K. Rowling', 55),
('Stephen King', 75),
('Agatha Christie', 80),
('Dan Brown', 60),
('J.R.R. Tolkien', 100),
('Sami Bowling', 66);

INSERT INTO books (id, author_id,content, titles, metadata) VALUES
(1, 2, 'This is a test test of the snippet function with multiple test words', ARRAY['test', 'snippet', 'function'], '{"test": "test"}'),
(1, 1, 'This is a final final of the snippet function with multiple final words', ARRAY['test', 'snippet', 'function'], '{"test": "test"}'),
(1, 6, 'This is a final test of the snippet function with multiple final words', ARRAY['test', 'snippet', 'function'], '{"test": "test"}'),
(2, 2, 'Another test of the snippet snippet function with repeated snippet words', ARRAY['test', 'test', 'function'], '{"test": "test"}'),
(3, 1, 'Yet another test test test of the function function function', ARRAY['test', 'snippet', 'test'], '{"test": "test"}'),
(4, 3, 'test Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur, vel illum qui dolorem eum fugiat quo voluptas nulla pariatur? test At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus. Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet ut et voluptates repudiandae sint et molestiae non recusandae. Itaque earum rerum hic tenetur a sapiente delectus, ut aut reiciendis voluptatibus maiores alias consequatur aut perferendis doloribus asperiores repellat. test', ARRAY['test', 'snippet', 'function'], '{"test": "test"}');

INSERT INTO reviews (book_id, review) VALUES
(1, 'This is a test review of the snippet function with multiple test words'),
(2, 'Another test review of the snippet snippet function with repeated snippet words'),
(3, 'Yet another test review of the function function function'),
(3, 'test review of the snippet function with multiple test words'),
(2, 'test review of the snippet snippet function with repeated snippet words'),
(1, 'test review of the function function function');

CREATE INDEX ON authors USING bm25 (
    id,
    name,
    age
) WITH (key_field = 'id');

CREATE INDEX ON books USING bm25 (
    id,
    author_id,
    content,
    titles
) WITH (key_field = 'id');

CREATE INDEX ON reviews USING bm25 (
    id,
    book_id,
    review
) WITH (key_field = 'id');
