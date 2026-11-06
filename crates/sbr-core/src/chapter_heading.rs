//! Port of `ChapterHeadingUtil` (titles from bibleocity.com, CC BY-NC-SA 4.0;
//! a few apocrypha headings from biblegateway.com).

mod data;

/// `getChapterTitle`; `None` where the original returns `null`.
pub fn chapter_title(book: i32, chapter: i32) -> Option<&'static str> {
    let key = (u16::try_from(book).ok()?, u16::try_from(chapter).ok()?);
    data::TITLES
        .binary_search_by_key(&key, |&(b, c, _)| (b, c))
        .ok()
        .map(|i| data::TITLES[i].2)
}

#[cfg(test)]
mod tests {
    #[test]
    fn lookup() {
        assert_eq!(super::chapter_title(1, 3), Some("3. The Fall of Man"));
        assert_eq!(super::chapter_title(87, 1), Some("1. Bel and the Dragon"));
        assert_eq!(super::chapter_title(1, 51), None);
        assert_eq!(super::chapter_title(-1, 1), None);
    }
}
