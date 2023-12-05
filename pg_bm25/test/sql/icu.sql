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

CREATE INDEX idx_arabic
ON arabic
USING bm25 ((arabic.*))
WITH (
    key_field='id',
    text_fields='{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
);

SELECT * FROM arabic WHERE arabic @@@ 'author:"محمد"';
SELECT * FROM arabic WHERE arabic @@@ 'title:"السوق"';
SELECT * FROM arabic WHERE arabic @@@ 'message:"في"';

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

CREATE INDEX idx_amharic
ON amharic
USING bm25 ((amharic.*))
WITH (
    key_field='id',
    text_fields='{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
);

SELECT * FROM amharic WHERE amharic @@@ 'author:"አለም"';
SELECT * FROM amharic WHERE amharic @@@ 'title:"ለመማር"';
SELECT * FROM amharic WHERE amharic @@@ 'message:"ዝናብ"';

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

CREATE INDEX idx_greek
ON greek
USING bm25 ((greek.*))
WITH (
    key_field='id',
    text_fields='{
        author: {tokenizer: {type: "icu"},},
        title: {tokenizer: {type: "icu"},},
        message: {tokenizer: {type: "icu"},}
    }'
);

SELECT * FROM greek WHERE greek @@@ 'author:"Σοφία"';
SELECT * FROM greek WHERE greek @@@ 'title:"επιτυχία"';
SELECT * FROM greek WHERE greek @@@ 'message:"συμβουλές"';