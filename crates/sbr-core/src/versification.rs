//! Port of `SwordVersification`: SWORD canons (adopted upstream from pysword).

mod data;

pub use data::VERSIFICATIONS;

#[derive(Debug)]
pub struct Book {
    pub name: &'static str,
    pub abbr: &'static str,
    /// Verse count of each chapter.
    pub chapters: &'static [u16],
}

#[derive(Debug)]
pub struct Versification {
    pub name: &'static str,
    pub ot: &'static [Book],
    pub nt: &'static [Book],
}

impl Versification {
    /// `testament` is `"ot"` or `"nt"`.
    pub fn testament(&self, testament: &str) -> Option<&'static [Book]> {
        match testament {
            "ot" => Some(self.ot),
            "nt" => Some(self.nt),
            _ => None,
        }
    }
}

/// `getVersifications`, in declaration order.
pub fn versifications() -> Vec<&'static str> {
    VERSIFICATIONS.iter().map(|v| v.name).collect()
}

pub fn get(name: &str) -> Option<&'static Versification> {
    VERSIFICATIONS.iter().find(|v| v.name == name)
}

/// `getTestamentOffsets`: `[book offsets, chapter offsets]` into a SWORD
/// `.bzs`/`.vss` index, as used by `offsets[t][1][offsets[t][0][book] + chapter]`.
pub fn testament_offsets(books: &[Book]) -> [Vec<i32>; 2] {
    let mut bks = vec![0];
    let mut cps = vec![0, 2];
    let mut bks_inc = 1;
    let mut cps_inc = 2;
    for book in books {
        bks.push(bks_inc);
        cps_inc += 1;
        cps.push(cps_inc);
        bks_inc += book.chapters.len() as i32 + 1;
        for &verses in book.chapters {
            cps_inc += i32::from(verses) + 1;
            cps.push(cps_inc);
        }
    }
    [bks, cps]
}

/// `getOffsets`: `[ot, nt]` testament offsets.
pub fn offsets(versification: &str) -> Option<[[Vec<i32>; 2]; 2]> {
    let v = get(versification)?;
    Some([testament_offsets(v.ot), testament_offsets(v.nt)])
}

/// `getBookIndex`: 1-based index of `book_abbr` (case-insensitive) in the testament, or -1.
pub fn book_index(versification: &str, book_abbr: &str, testament: &str) -> Option<i32> {
    let books = get(versification)?.testament(testament)?;
    let abbr = book_abbr.to_lowercase();
    Some(
        books
            .iter()
            .position(|b| b.abbr.to_lowercase() == abbr)
            .map_or(-1, |i| i as i32 + 1),
    )
}

/// `getBooks`: book abbreviations of the testament.
pub fn books(versification: &str, testament: &str) -> Option<Vec<&'static str>> {
    Some(
        get(versification)?
            .testament(testament)?
            .iter()
            .map(|b| b.abbr)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kjv_offsets_match_upstream_comment() {
        let [ot, _] = offsets("kjv").unwrap();
        assert_eq!(ot[0][..10], [0, 1, 52, 93, 121, 158, 193, 218, 240, 245]);
        assert_eq!(ot[1][..10], [0, 2, 3, 35, 61, 86, 113, 146, 169, 194]);
        assert_eq!(book_index("kjv", "gen", "ot"), Some(1));
        assert_eq!(book_index("kjv", "Matt", "nt"), Some(1));
        assert_eq!(book_index("kjv", "Matt", "ot"), Some(-1));
        assert_eq!(books("kjv", "nt").unwrap().len(), 27);
    }
}
