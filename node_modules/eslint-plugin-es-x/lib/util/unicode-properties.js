/* This file was generated with ECMAScript specifications. */
"use strict"

class DataSet {
    constructor(raw2018, raw2019, raw2020, raw2021, raw2022, raw2023, raw2024) {
        this._raw2018 = raw2018
        this._raw2019 = raw2019
        this._raw2020 = raw2020
        this._raw2021 = raw2021
        this._raw2022 = raw2022
        this._raw2023 = raw2023
        this._raw2024 = raw2024
    }

    get es2018() {
        return (
            this._set2018 || (this._set2018 = new Set(this._raw2018.split(" ")))
        )
    }

    get es2019() {
        return (
            this._set2019 || (this._set2019 = new Set(this._raw2019.split(" ")))
        )
    }

    get es2020() {
        return (
            this._set2020 || (this._set2020 = new Set(this._raw2020.split(" ")))
        )
    }

    get es2021() {
        return (
            this._set2021 || (this._set2021 = new Set(this._raw2021.split(" ")))
        )
    }

    get es2022() {
        return (
            this._set2022 || (this._set2022 = new Set(this._raw2022.split(" ")))
        )
    }

    get es2023() {
        return (
            this._set2023 || (this._set2023 = new Set(this._raw2023.split(" ")))
        )
    }

    get es2024() {
        return (
            this._set2024 || (this._set2024 = new Set(this._raw2024.split(" ")))
        )
    }
}

const gcNameSet = new Set(["General_Category", "gc"])
const scNameSet = new Set(["Script", "Script_Extensions", "sc", "scx"])
const gcValueSets = new DataSet(
    "C Cased_Letter Cc Cf Close_Punctuation Cn Co Combining_Mark Connector_Punctuation Control Cs Currency_Symbol Dash_Punctuation Decimal_Number Enclosing_Mark Final_Punctuation Format Initial_Punctuation L LC Letter Letter_Number Line_Separator Ll Lm Lo Lowercase_Letter Lt Lu M Mark Math_Symbol Mc Me Mn Modifier_Letter Modifier_Symbol N Nd Nl No Nonspacing_Mark Number Open_Punctuation Other Other_Letter Other_Number Other_Punctuation Other_Symbol P Paragraph_Separator Pc Pd Pe Pf Pi Po Private_Use Ps Punctuation S Sc Separator Sk Sm So Space_Separator Spacing_Mark Surrogate Symbol Titlecase_Letter Unassigned Uppercase_Letter Z Zl Zp Zs cntrl digit punct",
    "",
    "",
    "",
    "",
    "",
    "",
)
const scValueSets = new DataSet(
    "Adlam Adlm Aghb Ahom Anatolian_Hieroglyphs Arab Arabic Armenian Armi Armn Avestan Avst Bali Balinese Bamu Bamum Bass Bassa_Vah Batak Batk Beng Bengali Bhaiksuki Bhks Bopo Bopomofo Brah Brahmi Brai Braille Bugi Buginese Buhd Buhid Cakm Canadian_Aboriginal Cans Cari Carian Caucasian_Albanian Chakma Cham Cher Cherokee Common Copt Coptic Cprt Cuneiform Cypriot Cyrillic Cyrl Deseret Deva Devanagari Dsrt Dupl Duployan Egyp Egyptian_Hieroglyphs Elba Elbasan Ethi Ethiopic Geor Georgian Glag Glagolitic Gonm Goth Gothic Gran Grantha Greek Grek Gujarati Gujr Gurmukhi Guru Han Hang Hangul Hani Hano Hanunoo Hatr Hatran Hebr Hebrew Hira Hiragana Hluw Hmng Hung Imperial_Aramaic Inherited Inscriptional_Pahlavi Inscriptional_Parthian Ital Java Javanese Kaithi Kali Kana Kannada Katakana Kayah_Li Khar Kharoshthi Khmer Khmr Khoj Khojki Khudawadi Knda Kthi Lana Lao Laoo Latin Latn Lepc Lepcha Limb Limbu Lina Linb Linear_A Linear_B Lisu Lyci Lycian Lydi Lydian Mahajani Mahj Malayalam Mand Mandaic Mani Manichaean Marc Marchen Masaram_Gondi Meetei_Mayek Mend Mende_Kikakui Merc Mero Meroitic_Cursive Meroitic_Hieroglyphs Miao Mlym Modi Mong Mongolian Mro Mroo Mtei Mult Multani Myanmar Mymr Nabataean Narb Nbat New_Tai_Lue Newa Nko Nkoo Nshu Nushu Ogam Ogham Ol_Chiki Olck Old_Hungarian Old_Italic Old_North_Arabian Old_Permic Old_Persian Old_South_Arabian Old_Turkic Oriya Orkh Orya Osage Osge Osma Osmanya Pahawh_Hmong Palm Palmyrene Pau_Cin_Hau Pauc Perm Phag Phags_Pa Phli Phlp Phnx Phoenician Plrd Prti Psalter_Pahlavi Qaac Qaai Rejang Rjng Runic Runr Samaritan Samr Sarb Saur Saurashtra Sgnw Sharada Shavian Shaw Shrd Sidd Siddham SignWriting Sind Sinh Sinhala Sora Sora_Sompeng Soyo Soyombo Sund Sundanese Sylo Syloti_Nagri Syrc Syriac Tagalog Tagb Tagbanwa Tai_Le Tai_Tham Tai_Viet Takr Takri Tale Talu Tamil Taml Tang Tangut Tavt Telu Telugu Tfng Tglg Thaa Thaana Thai Tibetan Tibt Tifinagh Tirh Tirhuta Ugar Ugaritic Vai Vaii Wara Warang_Citi Xpeo Xsux Yi Yiii Zanabazar_Square Zanb Zinh Zyyy",
    "Dogr Dogra Gong Gunjala_Gondi Hanifi_Rohingya Maka Makasar Medefaidrin Medf Old_Sogdian Rohg Sogd Sogdian Sogo",
    "Elym Elymaic Hmnp Nand Nandinagari Nyiakeng_Puachue_Hmong Wancho Wcho",
    "Chorasmian Chrs Diak Dives_Akuru Khitan_Small_Script Kits Yezi Yezidi",
    "Cpmn Cypro_Minoan Old_Uyghur Ougr Tangsa Tnsa Toto Vith Vithkuqi",
    "Hrkt Katakana_Or_Hiragana Kawi Nag_Mundari Nagm Unknown Zzzz",
    "",
)
const binPropertySets = new DataSet(
    "AHex ASCII ASCII_Hex_Digit Alpha Alphabetic Any Assigned Bidi_C Bidi_Control Bidi_M Bidi_Mirrored CI CWCF CWCM CWKCF CWL CWT CWU Case_Ignorable Cased Changes_When_Casefolded Changes_When_Casemapped Changes_When_Lowercased Changes_When_NFKC_Casefolded Changes_When_Titlecased Changes_When_Uppercased DI Dash Default_Ignorable_Code_Point Dep Deprecated Dia Diacritic Emoji Emoji_Component Emoji_Modifier Emoji_Modifier_Base Emoji_Presentation Ext Extender Gr_Base Gr_Ext Grapheme_Base Grapheme_Extend Hex Hex_Digit IDC IDS IDSB IDST IDS_Binary_Operator IDS_Trinary_Operator ID_Continue ID_Start Ideo Ideographic Join_C Join_Control LOE Logical_Order_Exception Lower Lowercase Math NChar Noncharacter_Code_Point Pat_Syn Pat_WS Pattern_Syntax Pattern_White_Space QMark Quotation_Mark RI Radical Regional_Indicator SD STerm Sentence_Terminal Soft_Dotted Term Terminal_Punctuation UIdeo Unified_Ideograph Upper Uppercase VS Variation_Selector White_Space XIDC XIDS XID_Continue XID_Start space",
    "Extended_Pictographic",
    "",
    "EBase EComp EMod EPres ExtPict",
    "",
    "",
    "",
)
module.exports = {
    gcNameSet,
    scNameSet,
    gcValueSets,
    scValueSets,
    binPropertySets,
}
