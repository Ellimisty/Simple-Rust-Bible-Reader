//! Port of `Localization.cs`: UI strings and book names per locale.

use std::collections::HashMap;
use std::sync::OnceLock;

pub const LOCALES: [&str; 4] = ["en", "es", "ta", "pt"];

fn raw(lang: &str) -> &'static str {
    match lang {
        "es" => include_str!("../resources/locale/es.txt"),
        "ta" => include_str!("../resources/locale/ta.txt"),
        "pt" => include_str!("../resources/locale/pt.txt"),
        _ => include_str!("../resources/locale/en.txt"),
    }
}

/// `_loadLangLocalizations`: `key=value` lines; the value ends at the next `=`.
fn parse(text: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split('=');
        let k = parts.next().unwrap_or("").to_string();
        let v = parts.next().unwrap_or("").to_string();
        m.insert(k, v);
    }
    m
}

fn strings() -> &'static HashMap<&'static str, HashMap<String, String>> {
    static S: OnceLock<HashMap<&'static str, HashMap<String, String>>> = OnceLock::new();
    S.get_or_init(|| LOCALES.iter().map(|&l| (l, parse(raw(l)))).collect())
}

pub const SHORT_BOOK_NAMES: &[&str] = &[
    "Gen", "Exo", "Lev", "Num", "Deu", "Jos", "Jdg", "Rut", "1Sa", "2Sa", "1Ki", "2Ki", "1Ch",
    "2Ch", "Ezr", "Neh", "Est", "Job", "Psa", "Pro", "Ecc", "Sol", "Isa", "Jer", "Lam", "Eze",
    "Dan", "Hos", "Joe", "Amo", "Oba", "Jon", "Mic", "Nah", "Hab", "Zep", "Hag", "Zec", "Mal",
    "Mat", "Mar", "Luk", "Joh", "Act", "Rom", "1Co", "2Co", "Gal", "Eph", "Phi", "Col", "1Th",
    "2Th", "1Ti", "2Ti", "Tit", "Phm", "Heb", "Jam", "1Pe", "2Pe", "1Jo", "2Jo", "3Jo", "Jud",
    "Rev", "Jdt", "Wis", "Tob", "Sir", "Bar", "1Ma", "2Ma", "xDa", "xEs", "Man", "3Ma", "4Ma",
    "LJe", "1Es", "2Es", "Ode", "PsS", "Lao", "1En", "kGn", "Sus", "Bel", "Ps2", "Aza", "EsG",
    "DaG", "Jub", "4Ez", "5Ez", "6Ez", "Ps3", "2Ba", "4Ba", "LBa", "1Mq", "2Mq", "3Mq", "Rep",
];

pub const BOOK_NAME_KEYS: &[&str] = &[
    "Genesis",
    "Exodus",
    "Leviticus",
    "Numbers",
    "Deuteronomy",
    "Joshua",
    "Judges",
    "Ruth",
    "1_Samuel",
    "2_Samuel",
    "1_Kings",
    "2_Kings",
    "1_Chronicles",
    "2_Chronicles",
    "Ezra",
    "Nehemiah",
    "Esther",
    "Job",
    "Psalm",
    "Proverbs",
    "Ecclesiastes",
    "Song_of_Solomon",
    "Isaiah",
    "Jeremiah",
    "Lamentations",
    "Ezekiel",
    "Daniel",
    "Hosea",
    "Joel",
    "Amos",
    "Obadiah",
    "Jonah",
    "Micah",
    "Nahum",
    "Habakkuk",
    "Zephaniah",
    "Haggai",
    "Zechariah",
    "Malachi",
    "Matthew",
    "Mark",
    "Luke",
    "John",
    "Acts",
    "Romans",
    "1_Corinthians",
    "2_Corinthians",
    "Galatians",
    "Ephesians",
    "Philippians",
    "Colossians",
    "1_Thessalonians",
    "2_Thessalonians",
    "1_Timothy",
    "2_Timothy",
    "Titus",
    "Philemon",
    "Hebrews",
    "James",
    "1_Peter",
    "2_Peter",
    "1_John",
    "2_John",
    "3_John",
    "Jude",
    "Revelation",
    "Judit",
    "Wisdom",
    "Tobit",
    "Sirach",
    "Baruch",
    "1_Maccabees",
    "2_Maccabees",
    "Additions_to_Daniel",
    "Additions_to_Esther",
    "Prayer_of_Manasseh",
    "3_Maccabees",
    "4_Maccabees",
    "Letter_of_Jeremiah",
    "1_Esdras",
    "2_Esdras",
    "Odes",
    "Psalms_of_Solomon",
    "Epistle_to_the_Laodiceans",
    "1_Enoch",
    "kGen",
    "Susanna",
    "Bel_and_the_Dragon",
    "Psalm_151",
    "Prayer_of_Azariah",
    "Greek_Esther",
    "Greek_Daniel",
    "Jubilees",
    "Ezra_Apocalypse",
    "5_Ezra",
    "6_Ezra",
    "5_Apocryphal_Syriac_Psalms",
    "Syriac_Apocalypse_of_Baruch",
    "4_Baruch",
    "Letter_of_Baruch",
    "1_Meqabyan",
    "2_Meqabyan",
    "3_Meqabyan",
    "Reproof",
];

/// Localized UI string, falling back to English, then to the key.
pub fn get(lang: &str, key: &str) -> String {
    let s = strings();
    s.get(lang)
        .and_then(|m| m.get(key))
        .or_else(|| s["en"].get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `_loadLangBookNanes` for one locale.
pub fn book_names(lang: &str) -> &'static [String] {
    static B: OnceLock<HashMap<&'static str, Vec<String>>> = OnceLock::new();
    let all = B.get_or_init(|| {
        let s = strings();
        LOCALES
            .iter()
            .map(|&l| {
                let names = BOOK_NAME_KEYS
                    .iter()
                    .map(|k| {
                        s[l].get(*k)
                            .or_else(|| s["en"].get(*k))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();
                (l, names)
            })
            .collect()
    });
    all.get(lang).unwrap_or(&all["en"])
}
