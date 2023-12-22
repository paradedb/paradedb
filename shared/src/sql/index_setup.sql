-- English test table
CREATE TABLE one_republic_songs (
    song_id SERIAL PRIMARY KEY,
    title VARCHAR(100) NOT NULL,
    album VARCHAR(100),
    release_year INT,
    genre VARCHAR(50),
    description TEXT,
    lyrics TEXT
);

INSERT INTO one_republic_songs (title, album, release_year, genre, description, lyrics)
VALUES
    ('Secrets', 'Waking Up', 2009, 'Pop Rock', 'A brief description of the song Secrets.', 'I need another story,Something to get off my chest,My life gets kinda boring.'),
    ('Good Life', 'Waking Up', 2010, 'Pop', 'A brief description of the song Good Life.', 'Woke up in London yesterday,Found myself in the city near Piccadilly,Dont really know how I got here.'),
    ('If I Lose Myself', 'Native', 2013, 'Pop Rock', 'A brief description of the song If I Lose Myself.', 'I stared up at the sun,Thought of all of the people, places, and things Ive loved,I stared up just to see.'),
    ('Stop and Stare', 'Dreaming Out Loud', 2007, 'Alternative Rock', 'A brief description of the song Stop and Stare.', 'This town is colder now, I think its sick of us,Its time to make our move, Im shaking off the rust,Ive got my heart set on anywhere but here.'),
    ('All the Right Moves', 'Waking Up', 2009, 'Pop Rock', 'A brief description of the song All the Right Moves.', 'All the right moves in all the right places,So yeah, were going down,Theyve got all the right moves in all the right faces.'),
    ('Counting Stars', 'Native', 2013, 'Pop', 'A brief description of the song Counting Stars.',
        'Lately, Ive been, Ive been losing sleep,Dreaming about the things that we could be,But baby, Ive been, Ive been praying hard.'
    ),
    ('Apologize', 'Dreaming Out Loud', 2007, 'Pop Rock', 'A brief description of the song Apologize.',
        'Im holding on your rope, got me ten feet off the ground,And Im hearing what you say but I just cant make a sound,You tell me that you need me, then you go and cut me down.'
    );

CALL paradedb.create_bm25(
    index_name => 'one_republic_songs',
    table_name => 'one_republic_songs',
    key_field => 'song_id',
    text_fields => '{
        title: {},
        album: {},
        genre: {},
        description: {},
        lyrics: {}
    }',
    numeric_fields => '{
        release_year: {},
    }'
);


-- Chinese test table
CREATE TABLE IF NOT EXISTS chinese (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO chinese (author, title, message)
VALUES
    ('李华', '北京的新餐馆', '北京市中心新开了一家餐馆，以其现代设计和独特的菜肴选择而闻名。'),
    ('张伟', '篮球比赛回顾', '昨日篮球比赛精彩纷呈，尤其是最后时刻的逆转成为了比赛的亮点。'),
    ('王芳', '本地文化节', '本周末将举行一个地方文化节，预计将有各种食物和表演。');

CALL paradedb.create_bm25(
	index_name => 'chinese',
	table_name => 'chinese',
    key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "chinese_lindera"}, record: "position"},
        title: {tokenizer: {type: "chinese_lindera"}, record: "position"},
        message: {tokenizer: {type: "chinese_lindera"}, record: "position"}
    }'
);


-- Japanese test table
CREATE TABLE IF NOT EXISTS japanese (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO japanese (author, title, message)
VALUES
    ('佐藤健', '東京の新しいカフェ', '東京の中心部に新しいカフェがオープンしました。モダンなデザインとユニークなコーヒーが特徴です。'),
    ('鈴木一郎', 'サッカー試合レビュー', '昨日のサッカー試合では素晴らしいゴールが見られました。終了間際のドラマチックな展開がハイライトでした。'),
    ('高橋花子', '地元の祭り', '今週末に地元で祭りが開催されます。様々な食べ物とパフォーマンスが用意されています。');

CALL paradedb.create_bm25(
	index_name => 'japanese',
	table_name => 'japanese',
    key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "japanese_lindera"}, record: "position"},
        title: {tokenizer: {type: "japanese_lindera"}, record: "position"},
        message: {tokenizer: {type: "japanese_lindera"}, record: "position"}
    }'
);


-- Korean test table
CREATE TABLE IF NOT EXISTS korean (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO korean (author, title, message)
VALUES
    ('김민준', '서울의 새로운 카페', '서울 중심부에 새로운 카페가 문을 열었습니다. 현대적인 디자인과 독특한 커피 선택이 특징입니다.'),
    ('이하은', '축구 경기 리뷰', '어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.'),
    ('박지후', '지역 축제 개최 소식', '이번 주말 지역 축제가 열립니다. 다양한 음식과 공연이 준비되어 있어 기대가 됩니다.');

CALL paradedb.create_bm25(
	index_name => 'korean',
	table_name => 'korean',
	key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "korean_lindera"}, record: "position"},
        title: {tokenizer: {type: "korean_lindera"}, record: "position"},
        message: {tokenizer: {type: "korean_lindera"}, record: "position"}
    }'
);


-- Quoted table name test table
CREATE TABLE "Activity" (key SERIAL, name TEXT, age INTEGER);
INSERT INTO "Activity" (name, age) VALUES ('Alice', 29);
INSERT INTO "Activity" (name, age) VALUES ('Bob', 34);
INSERT INTO "Activity" (name, age) VALUES ('Charlie', 45);
INSERT INTO "Activity" (name, age) VALUES ('Diana', 27);
INSERT INTO "Activity" (name, age) VALUES ('Fiona', 38);
INSERT INTO "Activity" (name, age) VALUES ('George', 41);
INSERT INTO "Activity" (name, age) VALUES ('Hannah', 22);
INSERT INTO "Activity" (name, age) VALUES ('Ivan', 30);
INSERT INTO "Activity" (name, age) VALUES ('Julia', 25);
CALL paradedb.create_bm25(
	index_name => 'activity',
	table_name => 'Activity',
	key_field => 'key',
	text_fields => '{"name": {}}'
);
