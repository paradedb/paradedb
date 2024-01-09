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
        lyrics: {fast: true}
    }',
    numeric_fields => '{
        release_year: {},
    }'
);


-- Arabic test table
CREATE TABLE arabic (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO arabic (author, title, message)
VALUES
    ('فاطمة', 'رحلة إلى الشرق', 'في هذا المقال، سنستكشف رحلة مثيرة إلى الشرق ونتعرف على ثقافات مختلفة وتاريخها الغني'),
    ('محمد','رحلة إلى السوق مع أبي', 'مرحباً بك في المقالة الأولى. أتمنى أن تجد المحتوى مفيدًا ومثيرًا للاهتمام'),
    ('أحمد', 'نصائح للنجاح', 'هنا نقدم لك بعض النصائح القيمة لتحقيق النجاح في حياتك المهنية والشخصية. استفد منها وحقق أهدافك');

CALL paradedb.create_bm25(
	index_name => 'idx_arabic',
	table_name => 'arabic',
	key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
);



-- Amharic test table
CREATE TABLE amharic (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO amharic (author, title, message)
VALUES
    ('መሐመድ', 'መደመር ተጨማሪ', 'እንኳን ነበር በመደመር ተጨማሪ፣ በደስታ እና በልዩ ዝናብ ይከብዳል።'),
    ('ፋትስ', 'የምስሉ ማህበረሰብ', 'በዚህ ግዜ የምስሉ ማህበረሰብ እና እንደዚህ ዝናብ ይችላል።'),
    ('አለም', 'መረጃዎች ለመማር', 'እነዚህ መረጃዎች የምስሉ ለመማር በእያንዳንዱ ላይ ይመልከቱ።');

CALL paradedb.create_bm25(
	index_name => 'idx_amharic',
	table_name => 'amharic',
	key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
);


-- Greek test table
CREATE TABLE greek (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO greek (author, title, message)
VALUES
    ('Δημήτρης', 'Η πρώτη άρθρο', 'Καλώς ήρθες στο πρώτο άρθρο. Ελπίζω να βρεις το περιεχόμενο χρήσιμο και ενδιαφέρον.'),
    ('Σοφία', 'Ταξίδι στην Ανατολή', 'Σε αυτό το άρθρο, θα εξερευνήσουμε ένα συναρπαστικό ταξίδι στην Ανατολή και θα γνωρίσουμε διάφορες πολιτισμικές και ιστορικές πτυχές.'),
    ('Αλέξανδρος', 'Συμβουλές για την επιτυχία', 'Εδώ παρέχουμε μερικές πολύτιμες συμβουλές για την επίτευξη επιτυχίας στην επαγγελματική και προσωπική σας ζωή. Επωφεληθείτε από αυτές και επιτύχετε τους στόχους σας.');

CALL paradedb.create_bm25(
	index_name => 'idx_greek',
	table_name => 'greek',
	key_field => 'id',
    text_fields => '{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
)
