//! Turning a path a face was handed into a row of [`crate::layout::LAYOUT`] and what it captured.
//!
//! One function, and it is deliberately the only place in this crate that looks at a path's
//! text. A face passes whatever its kernel or its protocol gave it; what comes back is a row and
//! a [`Captures`], or nothing.
//!
//! # What is refused before anything is read
//!
//! A component that is `.`, `..` or empty, and any byte that is not ASCII. Both faces address
//! the same generated names, every one of which this crate wrote, so a path carrying a traversal
//! is either a face's bug or an attempt — and neither is a thing to resolve leniently. There is
//! no path here that reaches the file system, so this is not the *defence*; it is the
//! precondition that keeps the layout table the only description of the tree.

use crate::layout::{Generator, LAYOUT, Resolved, Route};

/// What a path filled its row's pattern in with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Captures {
    /// `NNNN`, the page ordinal counted from 1.
    pub page: Option<usize>,
    /// `DPI`, the resolution directory's dots per inch.
    pub dpi: Option<u32>,
    /// `NAME`, an entry a directory listing produced.
    pub name: Option<String>,
}

/// The components of a path, with the two shapes that are not names refused.
///
/// Returns `None` for a component that is empty, `.` or `..`, or that holds a byte outside
/// printable ASCII.
#[must_use]
pub fn components(path: &str) -> Option<Vec<&str>> {
    let trimmed = path.strip_prefix('/').unwrap_or(path);
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    let mut parts = Vec::new();
    for part in trimmed.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        if !part
            .bytes()
            .all(|byte| (0x20..0x7f).contains(&byte) && byte != b'/')
        {
            return None;
        }
        parts.push(part);
    }
    Some(parts)
}

/// The canonical spelling of a path: a leading solidus, no trailing one.
#[must_use]
pub fn canonical(components: &[&str]) -> String {
    if components.is_empty() {
        String::from("/")
    } else {
        format!("/{}", components.join("/"))
    }
}

/// The row a path names, and what it captured.
///
/// Structural only: it says the path *could* be that row, never that the document has such a
/// page, image or attachment. [`crate::Vfs`] asks the inventory that, because only the inventory
/// knows and it costs a read.
#[must_use]
pub fn resolve(path: &str) -> Option<Resolved> {
    let parts = components(path)?;
    let mut captures = Captures::default();
    let route = match parts.as_slice() {
        [] => row(Generator::Root),
        ["pages"] => row(Generator::PageOrdinals),
        ["pages", file] => {
            captures.page = Some(ordinal(file.strip_suffix(".pdf")?)?);
            row(Generator::ExtractedPage)
        }
        ["renders"] => row(Generator::Resolutions),
        ["renders", directory] => {
            captures.dpi = Some(dots_per_inch(directory)?);
            row(Generator::RenderOrdinals)
        }
        ["renders", directory, file] => {
            captures.dpi = Some(dots_per_inch(directory)?);
            captures.page = Some(ordinal(file.strip_suffix(".png")?)?);
            row(Generator::RenderedPage)
        }
        ["images"] => row(Generator::ImagePageOrdinals),
        ["images", directory] => {
            captures.page = Some(ordinal(directory)?);
            row(Generator::ImageInventory)
        }
        ["images", directory, file] => {
            captures.page = Some(ordinal(directory)?);
            captures.name = Some((*file).to_owned());
            row(Generator::ExtractedImage)
        }
        ["text"] => row(Generator::TextOrdinals),
        ["text", "document.txt"] => row(Generator::DocumentText),
        ["text", file] => {
            captures.page = Some(ordinal(file.strip_suffix(".txt")?)?);
            row(Generator::PageText)
        }
        ["attachments"] => row(Generator::AttachmentInventory),
        ["attachments", name] => {
            captures.name = Some((*name).to_owned());
            row(Generator::ExtractedAttachment)
        }
        ["meta"] => row(Generator::MetaNames),
        ["meta", "info.json"] => row(Generator::Information),
        ["meta", "xmp.xml"] => row(Generator::MetadataStream),
        ["meta", "outline.json"] => row(Generator::Outline),
        _ => return None,
    };
    Some((route?, captures))
}

/// The row a generator belongs to. Every generator appears exactly once in [`LAYOUT`], which
/// `the_table_names_every_generator_once` is what holds; `None` rather than a panic if one ever
/// does not, because a table with a hole in it is a finding rather than a crash.
fn row(generator: Generator) -> Option<&'static Route> {
    LAYOUT.iter().find(|route| route.generator == generator)
}

/// A zero-padded ordinal, counted from 1, with no sign, no space and no leading `+`.
///
/// Strict on purpose: `0007.pdf` and `7.pdf` must not both resolve, or one page would have two
/// names and a listing would disagree with a `stat`.
fn ordinal(text: &str) -> Option<usize> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: usize = text.parse().ok()?;
    (value > 0 && text.len() == width_of(value)).then_some(value)
}

/// How a listing spells `value` in this crate's ordinal directories: zero-padded to at least
/// four digits, and wider where the count needs it. [`crate::Vfs`] pads to the document's own
/// width; this is the *minimum*, and `ordinal` compares against the width the text itself has.
fn width_of(value: usize) -> usize {
    let digits = value.to_string().len();
    digits.max(MIN_ORDINAL_WIDTH)
}

/// RFC 0003 section 4's `0001.pdf`: four digits unless the page count needs more.
pub(crate) const MIN_ORDINAL_WIDTH: usize = 4;

/// The dots per inch a `renders/` sub-directory names: `150dpi`, `300dpi`.
fn dots_per_inch(text: &str) -> Option<u32> {
    let digits = text.strip_suffix("dpi")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: u32 = digits.parse().ok()?;
    (value > 0 && digits.len() == value.to_string().len()).then_some(value)
}

/// The name a page ordinal takes in a listing, at the document's own width.
#[must_use]
pub fn page_name(page: usize, width: usize, extension: &str) -> String {
    format!("{page:0width$}.{extension}")
}

/// The same name with no extension, which is what `images/` names a directory.
#[must_use]
pub fn page_name_stem(page: usize, width: usize) -> String {
    format!("{page:0width$}")
}

/// The name a `renders/` sub-directory takes.
#[must_use]
pub fn resolution_name(dpi: u32) -> String {
    format!("{dpi}dpi")
}

#[cfg(test)]
mod tests {
    use super::{Captures, components, ordinal, resolve};
    use crate::layout::{Generator, LAYOUT};

    #[test]
    fn the_table_names_every_generator_once() {
        for route in LAYOUT {
            let named = LAYOUT
                .iter()
                .filter(|other| other.generator == route.generator)
                .count();
            assert_eq!(named, 1, "{} names its generator twice", route.pattern);
        }
    }

    #[test]
    fn a_traversal_resolves_to_nothing() {
        for path in ["/pages/../etc", "/..", "/./pages", "/pages//0001.pdf"] {
            assert!(components(path).is_none(), "{path} resolved");
        }
    }

    #[test]
    fn an_ordinal_has_exactly_one_spelling() {
        assert_eq!(ordinal("0001"), Some(1));
        assert_eq!(ordinal("1"), None);
        assert_eq!(ordinal("00001"), None);
        assert_eq!(ordinal("0000"), None);
        assert_eq!(ordinal("12345"), Some(12345));
        assert_eq!(ordinal("+001"), None);
    }

    #[test]
    fn every_row_of_the_layout_resolves_from_a_path_that_matches_its_pattern() {
        let sample = |pattern: &str| -> String {
            pattern
                .replace("NNNN", "0007")
                .replace("DPI", "150dpi")
                .replace("NAME", "notes.txt")
        };
        for route in LAYOUT {
            let path = sample(route.pattern);
            let (found, _) = resolve(&path).unwrap_or_else(|| {
                panic!("{} did not resolve from {path}", route.pattern);
            });
            assert_eq!(found.generator, route.generator, "{path}");
        }
    }

    #[test]
    fn an_image_path_captures_its_page_and_the_name_the_transform_gave_it() {
        let (route, captures) = resolve("/images/0003/02.jp2").expect("resolves");
        assert_eq!(route.generator, Generator::ExtractedImage);
        assert_eq!(
            captures,
            Captures {
                page: Some(3),
                name: Some(String::from("02.jp2")),
                ..Captures::default()
            }
        );
    }

    #[test]
    fn a_mask_sidecar_is_a_name_like_any_other() {
        let (_, captures) = resolve("/images/0003/02.mask.png").expect("resolves");
        assert_eq!(captures.name.as_deref(), Some("02.mask.png"));
    }
}
