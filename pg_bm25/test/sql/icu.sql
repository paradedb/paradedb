-- ICU Arabic tokenizer
CREATE TABLE IF NOT EXISTS arabic (
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

SELECT * FROM idx_arabic.search('author:"محمد"');
SELECT * FROM idx_arabic.search('title:"السوق"');
SELECT * FROM idx_arabic.search('message:"في"');

-- ICU Amharic tokenizer
CREATE TABLE IF NOT EXISTS amharic (
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

SELECT * FROM idx_amharic.search('author:"አለም"');
SELECT * FROM idx_amharic.search('title:"ለመማር"');
SELECT * FROM idx_amharic.search('message:"ዝናብ"');

-- ICU Greek tokenizer
CREATE TABLE IF NOT EXISTS greek (
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
);

SELECT * FROM idx_greek.search('author:"Σοφία"');
SELECT * FROM idx_greek.search('title:"επιτυχία"');
SELECT * FROM idx_greek.search('message:"συμβουλές"');
