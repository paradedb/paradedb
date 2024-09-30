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
        'Im holding on your rope, got me ten feet off the ground,And Im hearing what you say but I just can't make a sound,You tell me that you need me, then you go and cut me down.'
    );

CALL paradedb.create_bm25(
    index_name => 'one_republic_songs',
    table_name => 'one_republic_songs',
    key_field => 'song_id',
    text_fields => paradedb.field('title') || paradedb.field('album') || paradedb.field('genre') || paradedb.field('description') || paradedb.field('lyrics', fast => true),
    numeric_fields => paradedb.field('release_year')
);
