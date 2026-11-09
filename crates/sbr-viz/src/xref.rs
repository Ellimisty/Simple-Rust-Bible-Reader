//! Cross-reference data in the OpenBible.info layout
//! (`From Verse<TAB>To Verse[-To Verse]<TAB>Votes`, OSIS ids).

use crate::bible::{osis_to_book, Verse};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossRef {
    pub from: Verse,
    pub to: Verse,
    /// Last verse of a range target (`== to` for a single verse).
    pub to_end: Verse,
    pub votes: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("line {line}: {msg}")]
    Parse { line: usize, msg: String },
}

fn parse_verse(s: &str) -> Option<Verse> {
    let mut it = s.split('.');
    let book = osis_to_book(it.next()?)?;
    let chapter = it.next()?.parse().ok()?;
    let verse = it.next()?.parse().ok()?;
    Some(Verse::new(book, chapter, verse))
}

pub fn parse(text: &str) -> Result<Vec<CrossRef>, Error> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') || line.starts_with("From Verse") {
            continue;
        }
        let err = |msg: &str| Error::Parse {
            line: i + 1,
            msg: format!("{msg}: {line:?}"),
        };
        let mut f = line.split('\t');
        let (Some(a), Some(b)) = (f.next(), f.next()) else {
            return Err(err("expected <from>\\t<to>[\\t<votes>]"));
        };
        let votes = match f.next() {
            Some(v) => v.trim().parse().map_err(|_| err("bad vote count"))?,
            None => 1,
        };
        let from = parse_verse(a).ok_or_else(|| err("bad verse"))?;
        let (to, to_end) = match b.split_once('-') {
            Some((s, e)) => (parse_verse(s), parse_verse(e)),
            None => (parse_verse(b), parse_verse(b)),
        };
        let (Some(to), Some(to_end)) = (to, to_end) else {
            return Err(err("bad verse"));
        };
        out.push(CrossRef {
            from,
            to,
            to_end,
            votes,
        });
    }
    Ok(out)
}

/// Loads a `.txt`/`.tsv` file, or the first `.txt` inside a `.zip`
/// (e.g. OpenBible's `cross-references.zip`).
pub fn load(path: &Path) -> Result<Vec<CrossRef>, Error> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
        let name = zip
            .file_names()
            .find(|n| n.ends_with(".txt") || n.ends_with(".tsv"))
            .map(str::to_owned)
            .ok_or(zip::result::ZipError::FileNotFound)?;
        let mut text = String::new();
        zip.by_name(&name)?.read_to_string(&mut text)?;
        return parse(&text);
    }
    parse(&String::from_utf8_lossy(&bytes))
}

/// Unordered pairs merged so `A→B` and `B→A` count once (max votes kept).
pub fn undirected(refs: &[CrossRef], min_votes: i32) -> Vec<CrossRef> {
    let mut m: HashMap<(Verse, Verse), CrossRef> = HashMap::new();
    for r in refs.iter().filter(|r| r.votes >= min_votes) {
        let key = if r.from <= r.to {
            (r.from, r.to)
        } else {
            (r.to, r.from)
        };
        m.entry(key)
            .and_modify(|e| e.votes = e.votes.max(r.votes))
            .or_insert(*r);
    }
    let mut v: Vec<_> = m.into_values().collect();
    v.sort_by_key(|r| (r.from, r.to));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openbible() {
        let t = "From Verse\tTo Verse\tVotes\t#www.openbible.info CC-BY\n\
                 Gen.1.1\t1Cor.8.6\t75\n\
                 Exod.20.3\tJas.2.10-Jas.2.11\t12\r\n\
                 1Cor.8.6\tGen.1.1\t3\n";
        let r = parse(t).unwrap();
        assert_eq!(r.len(), 3);
        assert_eq!(r[1].from, Verse::new(2, 20, 3));
        assert_eq!(r[1].to_end, Verse::new(59, 2, 11));
        assert_eq!(r[1].votes, 12);
        let u = undirected(&r, 0);
        assert_eq!(u.len(), 2);
        assert_eq!(u.iter().find(|x| x.from.book == 1).unwrap().votes, 75);
        assert!(parse("Foo.1.1\tGen.1.1\t1").is_err());
    }
}
