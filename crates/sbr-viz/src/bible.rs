//! Canon metadata used by the visualizations: book ids, genres and a linear
//! verse layout built from `bookid.xml` (via `sbr_core::format_util`).

use sbr_core::{chapter_heading, format_util, localization};
use std::sync::OnceLock;

pub const BOOKS: usize = 66;

/// OSIS book ids, as used by OpenBible.info and most cross-reference data sets.
pub const OSIS: [&str; BOOKS] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs",
    "1Chr", "2Chr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer",
    "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph",
    "Hag", "Zech", "Mal", "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal",
    "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas",
    "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev",
];

/// Compact labels for axes.
pub const ABBR: [&str; BOOKS] = [
    "Gen", "Ex", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sa", "2Sa", "1Kg", "2Kg", "1Ch",
    "2Ch", "Ezra", "Neh", "Est", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer", "Lam", "Ezek",
    "Dan", "Hos", "Joel", "Amos", "Obad", "Jon", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zech", "Mal",
    "Mat", "Mark", "Luke", "John", "Acts", "Rom", "1Co", "2Co", "Gal", "Eph", "Phil", "Col", "1Th",
    "2Th", "1Ti", "2Ti", "Titus", "Phm", "Heb", "Jas", "1Pe", "2Pe", "1Jn", "2Jn", "3Jn", "Jude",
    "Rev",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Genre {
    Law,
    History,
    Wisdom,
    MajorProphets,
    MinorProphets,
    Gospels,
    Acts,
    Pauline,
    General,
    Apocalyptic,
}

impl Genre {
    pub const ALL: [Genre; 10] = [
        Genre::Law,
        Genre::History,
        Genre::Wisdom,
        Genre::MajorProphets,
        Genre::MinorProphets,
        Genre::Gospels,
        Genre::Acts,
        Genre::Pauline,
        Genre::General,
        Genre::Apocalyptic,
    ];

    pub fn of(book: u8) -> Genre {
        match book {
            1..=5 => Genre::Law,
            6..=17 => Genre::History,
            18..=22 => Genre::Wisdom,
            23..=27 => Genre::MajorProphets,
            28..=39 => Genre::MinorProphets,
            40..=43 => Genre::Gospels,
            44 => Genre::Acts,
            45..=57 => Genre::Pauline,
            58..=65 => Genre::General,
            _ => Genre::Apocalyptic,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Genre::Law => "Law",
            Genre::History => "History",
            Genre::Wisdom => "Wisdom & Poetry",
            Genre::MajorProphets => "Major Prophets",
            Genre::MinorProphets => "Minor Prophets",
            Genre::Gospels => "Gospels",
            Genre::Acts => "Acts",
            Genre::Pauline => "Pauline Epistles",
            Genre::General => "General Epistles",
            Genre::Apocalyptic => "Revelation",
        }
    }

    /// Fallback label where [`Genre::name`] does not fit.
    pub fn short_name(self) -> &'static str {
        match self {
            Genre::Wisdom => "Wisdom",
            Genre::MajorProphets => "Major",
            Genre::MinorProphets => "Minor",
            Genre::Pauline => "Pauline",
            Genre::General => "General",
            Genre::Apocalyptic => "Rev",
            g => g.name(),
        }
    }

    pub fn books(self) -> std::ops::RangeInclusive<u8> {
        let first = (1..=BOOKS as u8).find(|&b| Genre::of(b) == self).unwrap();
        let last = (1..=BOOKS as u8).rfind(|&b| Genre::of(b) == self).unwrap();
        first..=last
    }
}

pub fn is_nt(book: u8) -> bool {
    book >= 40
}

pub fn abbr(book: u8) -> &'static str {
    ABBR[book as usize - 1]
}

pub fn name(book: u8) -> String {
    localization::book_names("en")[book as usize - 1].clone()
}

pub fn osis_to_book(id: &str) -> Option<u8> {
    OSIS.iter().position(|&o| o == id).map(|i| i as u8 + 1)
}

/// Resolves user input such as `Ex`, `Exod`, `Exodus`, `jas` or `59`.
pub fn parse_book(s: &str) -> Option<u8> {
    let t = s.trim();
    if let Ok(n) = t.parse::<u8>() {
        return (1..=BOOKS as u8).contains(&n).then_some(n);
    }
    let l = t.to_lowercase();
    let pos = |list: &[&str]| list.iter().position(|x| x.to_lowercase() == l);
    if let Some(i) = pos(&ABBR).or_else(|| pos(&OSIS)) {
        return Some(i as u8 + 1);
    }
    if let Some(i) = localization::book_names("en")
        .iter()
        .take(BOOKS)
        .position(|n| n.to_lowercase() == l)
    {
        return Some(i as u8 + 1);
    }
    let n = format_util::get_book_no(t);
    (1..=BOOKS as i32).contains(&n).then_some(n as u8)
}

/// `ChapterHeadingUtil` title without its `"N. "`/`"N-M."` prefix and with
/// the upstream's mis-decoded punctuation (`'\u{80}\u{99}` …) repaired.
pub fn chapter_title(book: u8, chapter: u16) -> Option<String> {
    let t = chapter_heading::chapter_title(book as i32, chapter as i32)?;
    let t = match t.split_once('.') {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit() || c == '-') => rest,
        _ => t,
    };
    let t = t
        .trim()
        .replace("'\u{80}\u{99}", "\u{2019}")
        .replace("'\u{80}\u{9c}", "\u{201c}")
        .replace("'\u{80}\u{9d}", "\u{201d}")
        .replace("'\u{80}\u{94}", "\u{2014}")
        .replace("'\u{bd}", "\u{bd}");
    Some(
        t.chars()
            .filter(|c| !('\u{80}'..='\u{9f}').contains(c))
            .collect(),
    )
}

/// Verses laid end to end, Genesis 1:1 = 0.
pub struct Layout {
    /// `verses[book-1][chapter-1]`
    pub verses: Vec<Vec<u32>>,
    /// Global index of each chapter's first verse.
    chapter_start: Vec<Vec<u32>>,
    book_start: Vec<u32>,
    pub total: u32,
}

impl Layout {
    pub fn get() -> &'static Layout {
        static L: OnceLock<Layout> = OnceLock::new();
        L.get_or_init(|| {
            let mut verses = Vec::new();
            let mut chapter_start = Vec::new();
            let mut book_start = Vec::new();
            let mut total = 0u32;
            for b in 1..=BOOKS as i32 {
                book_start.push(total);
                let mut vs = Vec::new();
                let mut cs = Vec::new();
                for c in 1..=format_util::get_chapter_count(b) {
                    let n = format_util::get_verse_count(b, c).max(1) as u32;
                    cs.push(total);
                    vs.push(n);
                    total += n;
                }
                verses.push(vs);
                chapter_start.push(cs);
            }
            Layout {
                verses,
                chapter_start,
                book_start,
                total,
            }
        })
    }

    pub fn chapters(&self, book: u8) -> usize {
        self.verses[book as usize - 1].len()
    }

    pub fn chapter_verses(&self, book: u8, chapter: u16) -> u32 {
        self.verses[book as usize - 1][chapter as usize - 1]
    }

    pub fn book_verses(&self, book: u8) -> u32 {
        self.verses[book as usize - 1].iter().sum()
    }

    pub fn book_start(&self, book: u8) -> u32 {
        self.book_start[book as usize - 1]
    }

    pub fn chapter_start(&self, book: u8, chapter: u16) -> u32 {
        self.chapter_start[book as usize - 1][chapter as usize - 1]
    }

    /// Global chapter number, Genesis 1 = 0.
    pub fn chapter_index(&self, book: u8, chapter: u16) -> usize {
        self.verses[..book as usize - 1]
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            + chapter as usize
            - 1
    }

    pub fn total_chapters(&self) -> usize {
        self.verses.iter().map(Vec::len).sum()
    }

    /// Offset (in verses, fractional at the verse centre) of a verse from the
    /// start of its book; out-of-range chapters/verses are clamped.
    pub fn offset_in_book(&self, v: Verse) -> f64 {
        let vs = &self.verses[v.book as usize - 1];
        let c = (v.chapter as usize).clamp(1, vs.len());
        let before: u32 = vs[..c - 1].iter().sum();
        let n = vs[c - 1];
        before as f64 + (v.verse as u32).clamp(1, n) as f64 - 0.5
    }

    pub fn global(&self, v: Verse) -> f64 {
        self.book_start(v.book) as f64 + self.offset_in_book(v)
    }

    /// Clamped `(book, chapter)` of a verse.
    pub fn clamp_chapter(&self, v: Verse) -> u16 {
        (v.chapter as usize).clamp(1, self.chapters(v.book)) as u16
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Verse {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
}

impl Verse {
    pub fn new(book: u8, chapter: u16, verse: u16) -> Verse {
        Verse {
            book,
            chapter,
            verse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn books() {
        assert_eq!(parse_book("Ex"), Some(2));
        assert_eq!(parse_book("exodus"), Some(2));
        assert_eq!(parse_book("Jas"), Some(59));
        assert_eq!(parse_book("James"), Some(59));
        assert_eq!(parse_book("1John"), Some(62));
        assert_eq!(parse_book("66"), Some(66));
        assert_eq!(parse_book("nope"), None);
        assert_eq!(osis_to_book("Phlm"), Some(57));
        assert_eq!(Genre::of(44), Genre::Acts);
        assert_eq!(Genre::Pauline.books(), 45..=57);
    }

    #[test]
    fn layout() {
        let l = Layout::get();
        assert_eq!(l.total, 31102);
        assert_eq!(l.total_chapters(), 1189);
        assert_eq!(l.chapters(2), 40);
        assert_eq!(l.chapter_index(2, 1), 50);
        assert_eq!(l.offset_in_book(Verse::new(1, 2, 1)), 31.5);
        assert_eq!(l.offset_in_book(Verse::new(1, 1, 99)), 30.5);
        assert_eq!(chapter_title(2, 1).as_deref(), Some("Slavery in Egypt"));
        assert_eq!(chapter_title(2, 7).as_deref(), Some("The Ten Plagues"));
        assert_eq!(
            chapter_title(23, 30).as_deref(),
            Some("Judah\u{2019}s Alliance With Egypt")
        );
        assert_eq!(
            chapter_title(3, 10).as_deref(),
            Some("Strange Fire\u{2014}Nadab & Abihu")
        );
    }
}
