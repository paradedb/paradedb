// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;
use tantivy::tokenizer::Language;

use tokenizers::manager::language_to_str;

#[rstest]
fn language_stem_search_test(mut conn: PgConnection) {
    // Define languages and corresponding test data
    let languages = vec![
        (
            Language::Arabic,
            "('محمد','رحلة إلى السوق مع أبي', 'مرحباً بك في المقالة الأولى. أتمنى أن تجد المحتوى مفيدًا ومثيرًا للاهتمام'),
            ('فاطمة', 'رحلة إلى الشرق', 'في هذا المقال، سنستكشف رحلة مثيرة إلى الشرق ونتعرف على ثقافات مختلفة وتاريخها الغني'),
            ('أحمد', 'نصائح للنجاح', 'هنا نقدم لك بعض النصائح القيمة لتحقيق النجاح في حياتك المهنية والشخصية. استفد منها وحقق أهدافك')",
            "محمد",
            "شرق",
            "هنا",
        ),
        (
            Language::Danish,
            "('Mette Hansen', 'Ny Bogudgivelse', 'Spændende ny bog udgivet af anerkendt forfatter.'),
            ('Lars Jensen', 'Teknologikonference Højdepunkter', 'Højdepunkter fra den seneste teknologikonference.'),
            ('Anna Nielsen', 'Lokal Kulturfestival', 'Der afholdes en lokal kulturfestival i weekenden med forventede madboder og forestillinger.')",
            "met",
            "højdepunk",
            "weekend",
        ),
        (
            Language::Dutch,
            " ('Pieter de Vries', 'Nieuw Boek Uitgebracht', 'Spannend nieuw boek uitgebracht door een bekende auteur.'),
            ('Annelies Bakker', 'Technologie Conferentie Hoogtepunten', 'Hoogtepunten van de laatste technologie conferentie.'),
            ('Jan Jansen', 'Lokale Culturele Festival', 'Dit weekend wordt er een lokaal cultureel festival gehouden met verwachte eetkraampjes en optredens.')",
            "vries",
            "hoogtepunt",
            "lokal",
        ),
        (
            Language::English,
            "('John Doe', 'New Book Release', 'Exciting new book released by renowned author.'),
            ('Jane Smith', 'Tech Conference Highlights', 'Highlights from the latest tech conference.'),
            ('Michael Brown', 'Local Charity Event', 'Upcoming charity event featuring local artists and performers.')",
            "john",
            "confer",
            "perform",
        ),
        (
            Language::Finnish,
            "('Matti Virtanen', 'Uusi Kirjan Julkaisu', 'Jännittävä uusi kirja julkaistu tunnetulta kirjailijalta.'),
            ('Anna Lehtonen', 'Teknologiakonferenssin Keskustelut', 'Viimeisimmän teknologiakonferenssin keskustelut ja huomiot.'),
            ('Juha Mäkinen', 'Paikallinen Kulttuuritapahtuma', 'Viikonloppuna järjestetään paikallinen kulttuuritapahtuma, jossa on odotettavissa erilaisia ruokakojuja ja esityksiä.')",
            "mat",
            "keskustelu",
            "järjest",
        ),
        (
            Language::French,
            "('Jean Dupont', 'Nouvelle Publication', 'Nouveau livre passionnant publié par un auteur renommé.'),
             ('Marie Leclerc', 'Points Forts de la Conférence Technologique', 'Points forts de la dernière conférence technologique.'),
             ('Pierre Martin', 'Festival Culturel Local', 'Ce week-end se tiendra un festival culturel local avec des stands de nourriture et des spectacles prévus.')",
            "dupont",
            "technolog",
            "tiendr",
        ),
        (
            Language::German,
            "('Hans Müller', 'Neue Buchveröffentlichung', 'Spannendes neues Buch veröffentlicht von einem bekannten Autor.'),
             ('Anna Schmidt', 'Highlights der Technologiekonferenz', 'Höhepunkte der letzten Technologiekonferenz.'),
             ('Michael Wagner', 'Lokales Kulturfestival', 'Am Wochenende findet ein lokales Kulturfestival statt, mit erwarteten Essensständen und Auftritten.')",
            "mull",
            "technologiekonferenz",
            "essensstand",
        ),
        (
            Language::Greek,
            "('Γιάννης Παπαδόπουλος', 'Νέα Έκδοση Βιβλίου', 'Συναρπαστικό νέο βιβλίο κυκλοφόρησε από γνωστό συγγραφέα.'),
             ('Αννα Στεφανίδου', 'Κορυφαίες Στιγμές της Τεχνολογικής Διάσκεψης', 'Κορυφαίες στιγμές από την τελευταία τεχνολογική διάσκεψη.'),
             ('Μιχάλης Παπαδόπουλος', 'Τοπικό Πολιτιστικό Φεστιβάλ', 'Το Σαββατοκύριακο θα πραγματοποιηθεί τοπικό πολιτιστικό φεστιβάλ με αναμενόμενα περίπτερα φαγητού και εμφανίσεις.')",
            "Παπαδόπουλος",
            "διασκεψ",
            "σαββατοκυριακ",
        ),
        (
            Language::Hungarian,
            "('János Kovács', 'Új Könyv Megjelenése', 'Izgalmas új könyv jelent meg egy ismert szerzőtől.'),
             ('Anna Nagy', 'Technológiai Konferencia Kiemelkedői', 'A legutóbbi technológiai konferencia kiemelkedő pillanatai.'),
             ('Gábor Tóth', 'Helyi Kulturális Fesztivál', 'Hétvégén helyi kulturális fesztivált rendeznek, várhatóan ételstandokkal és előadásokkal.')",
            "jános",
            "kiemelkedő",
            "várható",
        ),
        (
            Language::Italian,
            "('Giuseppe Rossi', 'Nuova Pubblicazione Libro', 'Nuovo libro emozionante pubblicato da un autore famoso.'),
             ('Maria Bianchi', 'Highlights della Conferenza Tecnologica', 'I momenti salienti della recente conferenza tecnologica.'),
             ('Luca Verdi', 'Festival Culturale Locale', 'Questo fine settimana si terrà un festival culturale locale, con previsti stand gastronomici e spettacoli.')",
            "ross",
            "conferent",
            "gastronom",
        ),
        (
            Language::Norwegian,
            "('Ole Hansen', 'Ny Bokutgivelse', 'Spennende ny bok utgitt av en kjent forfatter.'),
             ('Kari Olsen', 'Høydepunkter fra Teknologikonferansen', 'Høydepunkter fra den siste teknologikonferansen.'),
             ('Per Johansen', 'Lokal Kulturfestival', 'Denne helgen arrangeres det en lokal kulturfestival med forventede matboder og forestillinger.')",
            "ole",
            "høydepunkt",
            "forestilling",
        ),
        (
            Language::Portuguese,
            "('João Silva', 'Novo Lançamento de Livro', 'Novo livro emocionante lançado por um autor famoso.'),
             ('Maria Santos', 'Destaques da Conferência de Tecnologia', 'Os destaques da última conferência de tecnologia.'),
             ('Pedro Oliveira', 'Festival Cultural Local', 'Neste fim de semana será realizado um festival cultural local, com barracas de comida e apresentações esperadas.')",
            "joã",
            "conferent",
            "será",
        ),
        (
            Language::Romanian,
            "('Ion Popescu', 'Nouă Publicație de Carte', 'O carte nouă și captivantă publicată de un autor renumit.'),
             ('Ana Ionescu', 'Momentele Cheie ale Conferinței Tehnologice', 'Cele mai importante momente ale ultimei conferințe tehnologice.'),
             ('Mihai Radu', 'Festival Cultural Local', 'În acest weekend va avea loc un festival cultural local, cu standuri de mâncare și spectacole programate.')",
            "popescu",
            "moment",
            "mânc",
        ),
        (
            Language::Russian,
            "('Иван Иванов', 'Новое издание книги', 'Увлекательная новая книга, выпущенная известным автором.'),
             ('Мария Петрова', 'Основные моменты технологической конференции', 'Основные моменты последней технологической конференции.'),
             ('Алексей Сидоров', 'Местный культурный фестиваль', 'В этот уикенд состоится местный культурный фестиваль с предполагаемыми палатками с едой и выступлениями.')",
            "иван",
            "технологическ",
            "культурн",
        ),
        (
            Language::Spanish,
            "('Juan Pérez', 'Nuevo Lanzamiento de Libro', 'Nuevo libro emocionante publicado por un autor famoso.'),
             ('María García', 'Aspectos Destacados de la Conferencia Tecnológica', 'Los momentos más destacados de la última conferencia tecnológica.'),
             ('Carlos Martínez', 'Festival Cultural Local', 'Este fin de semana se llevará a cabo un festival cultural local, con puestos de comida y actuaciones programadas.')",
            "pérez",
            "destac",
            "com",
        ),
        (
            Language::Swedish,
            "('Anna Andersson', 'Ny Bokutgivning', 'Spännande ny bok utgiven av en känd författare.'),
             ('Johan Eriksson', 'Höjdpunkter från Teknologikonferensen', 'Höjdpunkter från den senaste teknologikonferensen.'),
             ('Emma Nilsson', 'Lokalt Kulturfestival', 'Den här helgen hålls en lokal kulturfestival med förväntade matstånd och föreställningar.')",
            "ann",
            "höjdpunk",
            "föreställning",
        ),
        (
            Language::Tamil,
            "('சுப்ரமணியம் சுப்பிரமணியம்', 'புதிய புத்தக வெளியிடுதல்', 'ஒரு பிரபல எழுத்தாளரால் வெளியிடப்பட்ட புதிய புத்தகம்.'),
             ('லக்ஷ்மி சுந்தரம்', 'தொழில்நுட்ப மாநாடு முக்கியப்பட்டவை', 'கடைசி தொழில்நுட்ப மாநாட்டின் முக்கிய நிகழ்வுகள்.'),
             ('அருணா குமார்', 'உள்ளூர் கலாச்சார திருவிழா', 'இந்த வாரம் ஒரு உள்ளூர் கலாச்சார திருவிழா நடைபெறும், எங்களுக்கு உண்டாக்கப்பட்ட உணவு முன்னேற்றங்களுடன்.')",
            "சுப்பிரமணியம",
            "மாநாடு",
            "திருவிழா",
        ),
        (
            Language::Turkish,
            "('Ahmet Yılmaz', 'Yeni Kitap Yayınlandı', 'Ünlü bir yazar tarafından heyecan verici yeni bir kitap yayınlandı.'),
            ('Ayşe Kaya', 'Teknoloji Konferansının Öne Çıkanları', 'Son teknoloji konferansının öne çıkanları.'),
            ('Mehmet Demir', 'Yerel Kültür Festivali', 'Bu hafta sonu yerel bir kültür festivali düzenlenecek, yiyecek standları ve planlanmış gösterilerle.')",
            "yılmaz",
            "konferansı",
            "göster",
        )
    ];

    for (language, data, author_query, title_query, message_query) in languages {
        // Prepare test data setup for each language
        let language_str = language_to_str(&language);
        let setup_query = format!(
            r#"
            DROP TABLE IF EXISTS test_table;
            CREATE TABLE IF NOT EXISTS test_table(
                id SERIAL PRIMARY KEY,
                author TEXT,
                title TEXT,
                message TEXT
            );
            INSERT INTO test_table (author, title, message)
            VALUES {};
            CALL paradedb.create_bm25(
                index_name => 'stem_test',
                table_name => 'test_table',
                key_field => 'id',
                text_fields => '{{
                    author: {{tokenizer: {{type: "stem", language:"{}"}}, record: "position"}},
                    title: {{tokenizer: {{type: "stem", language:"{}"}}, record: "position"}},
                    message: {{tokenizer: {{type: "stem", language:"{}"}}, record: "position"}}
                }}'
            );"#,
            data, language_str, language_str, language_str
        );

        setup_query.execute(&mut conn);

        let author_search_query = format!(
            "SELECT id FROM stem_test.search('author:{}', stable_sort => true)",
            author_query
        );
        let title_search_query = format!(
            "SELECT id FROM stem_test.search('title:{}', stable_sort => true)",
            title_query
        );
        let message_search_query = format!(
            "SELECT id FROM stem_test.search('message:{}', stable_sort => true)",
            message_query
        );

        let row: (i32,) = author_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 1);

        let row: (i32,) = title_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 2);

        let row: (i32,) = message_search_query.fetch_one(&mut conn);
        assert_eq!(row.0, 3);

        r#"
        CALL paradedb.drop_bm25('stem_test');
        DROP TABLE IF EXISTS test_table;"#
            .execute(&mut conn);
    }
}
