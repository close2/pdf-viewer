//! A `/BaseFont` whose bytes are not ASCII, read as the characters it names (ADR 3117).
//!
//! ISO 32000-2 §7.3.5 makes a name "an atomic symbol uniquely defined by a sequence of any
//! characters (8-bit values) except null", and names `/BaseFont` as the case where a processor
//! reads one as text anyway:
//!
//! > In such situations, the sequence of bytes making up the name object should be interpreted
//! > according to UTF-8, a variable-length byte-encoded representation of Unicode in which the
//! > printable ASCII characters have the same representations as in ASCII.
//!
//! So a name that is UTF-8 is read as UTF-8, which is the standard's reading. A name that is not
//! has departed from a "should", and the standard then says nothing of what its bytes mean.
//! **What follows is this tree's choice**: a composite font's character collection (§9.7.3's
//! `/CIDSystemInfo`) is tied by Table 116 to a language, and Table 116 also lists the byte
//! encodings that language's documents were written in — GBK, Big Five, Shift-JIS and EUC-JP,
//! Unified Hangul Code — as predefined `CMap`s of that collection. A name such a producer wrote in
//! its own system's encoding is read through those `CMap`s and the collection's `-UCS2` table
//! (§9.10.2's third method), and the reading is used only where it decides something: where it
//! names a style of type ([`class_of`]), and every reading that names one names the same.

use super::Collection;

/// Table 116's horizontal `CMap`s for the byte encodings of a collection's language, each with
/// the collection whose `-UCS2` table names its CIDs' characters.
///
/// One encoding per national character set, and the widest of each family: GBK's `CMap` reads
/// GB 2312 too, and Microsoft's Unified Hangul Code reads KS X 1001's EUC form. Japanese has two
/// unrelated byte encodings and Traditional Chinese two Big Five extensions, so those name both.
/// A Korean font of Adobe-KR is read through Adobe-Korea1's `CMap`, because Table 116's Korean
/// heading lists no byte encoding for Adobe-KR; the name's characters are the same either way.
const fn readings(collection: Collection) -> &'static [(&'static str, &'static str)] {
    match collection {
        Collection::Gb1 => &[("GBK-EUC-H", "GB1")],
        Collection::Cns1 => &[("ETen-B5-H", "CNS1"), ("HKscs-B5-H", "CNS1")],
        Collection::Japan1 => &[("90ms-RKSJ-H", "Japan1"), ("EUC-H", "Japan1")],
        Collection::Korea1 | Collection::Kr => &[("KSCms-UHC-H", "Korea1")],
    }
}

/// The characters outside ASCII that a `/BaseFont`'s bytes name, where they can be read.
///
/// UTF-8 first, as §7.3.5 asks, for any font; then, for a font naming a collection, each of
/// [`readings`] in turn, kept only where it names a style ([`class_of`]) and no other reading names
/// a different one. `None` for an ASCII name, and for bytes nothing reads.
pub(super) fn read(bytes: &[u8], collection: Option<Collection>) -> Option<String> {
    if bytes.is_ascii() {
        return None;
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Some(text.chars().filter(|c| !c.is_ascii()).collect());
    }
    let mut agreed: Option<(String, Class)> = None;
    for (cmap, ordering) in readings(collection?) {
        let Some(text) = through(bytes, cmap, ordering) else {
            continue;
        };
        let Some(class) = class_of(&text) else {
            continue;
        };
        match &agreed {
            None => agreed = Some((text, class)),
            Some((_, held)) if *held == class => {}
            // Two encodings read two different styles out of one name: neither is the producer's
            // statement more than the other, so the name says nothing.
            Some(_) => return None,
        }
    }
    agreed.map(|(text, _)| text)
}

/// `bytes` read through the predefined `CMap` `cmap` and the `Adobe-<ordering>-UCS2` table: the
/// characters outside ASCII, or `None` where a code maps to no CID, to CID 0, or to no character.
fn through(bytes: &[u8], cmap: &str, ordering: &str) -> Option<String> {
    let cmap = crate::predefined::cmap(cmap)?;
    let table = crate::predefined::cid_to_unicode("Adobe", ordering)?;
    let mut rest = bytes;
    let mut text = String::new();
    while !rest.is_empty() {
        let code = cmap.next_code(rest);
        let cid = cmap.cid(code).filter(|cid| *cid != 0)?;
        let character = table.char_for(cid)?;
        if !character.is_ascii() {
            text.push(character);
        }
        rest = rest.get(usize::from(code.length())..)?;
    }
    (!text.is_empty()).then_some(text)
}

/// The style of type a name read as characters says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Mincho, Song, Ming, Myeongjo or Batang: the serifed styles.
    Serif,
    /// Hei, Gothic, Dotum or Gulim: the sans-serif styles.
    Sans,
}

/// The serifed styles' names as their own scripts write them: Japanese 明朝 (Mincho), Chinese 宋
/// (Song, in 宋体, 宋體 and 仿宋) and 明體 or 明体 (Ming), Korean 명조 (Myeongjo) and 바탕 (Batang).
pub(super) const SERIF_WORDS: &[&str] = &["明朝", "宋", "明體", "明体", "명조", "바탕"];

/// The sans-serif styles' names as their own scripts write them: Chinese 黑 (Hei, in 黑体, 黑體,
/// 正黑 and 雅黑), Japanese ゴシック (Gothic) and its short form 角ゴ, Korean 고딕 (Gothic), 돋움
/// (Dotum) and 굴림 (Gulim).
pub(super) const SANS_WORDS: &[&str] = &["黑", "ゴシック", "角ゴ", "고딕", "돋움", "굴림"];

/// [`Class::Serif`] or [`Class::Sans`] where `text` names exactly one of the two.
fn class_of(text: &str) -> Option<Class> {
    let serif = SERIF_WORDS.iter().any(|word| text.contains(word));
    let sans = SANS_WORDS.iter().any(|word| text.contains(word));
    match (serif, sans) {
        (true, false) => Some(Class::Serif),
        (false, true) => Some(Class::Sans),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Collection, read};

    /// `issue3521`'s 黑体 in GBK, `BA DA CC E5`, which is not UTF-8: read through GBK's `CMap` and
    /// Adobe-GB1's table it is Hei, a sans style. Planted: with the collection's readings emptied,
    /// the name reads as nothing.
    #[test]
    fn a_gbk_name_is_read_through_its_collections_cmap() {
        let hei = [0xBA, 0xDA, 0xCC, 0xE5];
        assert_eq!(read(&hei, Some(Collection::Gb1)).as_deref(), Some("黑体"));
        let bold: Vec<u8> = hei.iter().copied().chain(*b",Bold").collect();
        assert_eq!(read(&bold, Some(Collection::Gb1)).as_deref(), Some("黑体"));
        // With no collection there is no encoding to read GBK by.
        assert_eq!(read(&hei, None), None);
    }

    /// §7.3.5's reading, for any font: a UTF-8 name is its characters.
    #[test]
    fn a_utf8_name_is_read_as_utf8() {
        assert_eq!(read("MS 明朝".as_bytes(), None).as_deref(), Some("明朝"));
        assert_eq!(read(b"SimSun", Some(Collection::Gb1)), None);
    }

    /// Shift-JIS 明朝 (`96 BE 92 A9`) under Adobe-Japan1, and Unified Hangul Code 바탕 (`B9 D9 C5
    /// C1`) under Adobe-KR, which reads through Adobe-Korea1's encoding.
    #[test]
    fn a_shift_jis_and_a_uhc_name_are_read() {
        assert_eq!(
            read(
                &[0x82, 0x6C, 0x82, 0x72, 0x96, 0xBE, 0x92, 0xA9],
                Some(Collection::Japan1)
            )
            .as_deref()
            .map(|text| text.contains("明朝")),
            Some(true)
        );
        assert_eq!(
            read(&[0xB9, 0xD9, 0xC5, 0xC1], Some(Collection::Kr)).as_deref(),
            Some("바탕")
        );
    }
}
