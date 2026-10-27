//! §13.7.2.3.2's assets: the embedded files a rich media annotation carries, as attachments.
//!
//! ISO 32000-2 §13.7.2.3.2:
//!
//! > The Assets entry takes a name tree of embedded file specification dictionaries .
//!
//! Each value is §7.11.3's file specification and its `/EF` §7.11.4's stream, so an asset is an
//! [`Attachment`] exactly as an `/EmbeddedFiles` entry is, filed under the tree's key. `CLAUDE.md`'s
//! clause 13 entry excludes playing the media and admits handing the file over; the owner's A631
//! puts the assets on the attachments' side of that line — listed and saved under the reader's
//! existing level for embedded files, never handed to a player, because a player handed a Flash
//! asset can do nothing with it and a person who saves one can (ADR 2399).
//!
//! **Per annotation, and that is principle 2.** An annotation sits on a page, and a list of every
//! page's assets would walk the whole page tree when a document opens — what ADR 0295 measured for
//! §12.5.6.15's annotations and `CLAUDE.md` forbids on the launch path. So the reader asks of one
//! annotation it already holds: the one clicked, or those on the page in view.
//!
//! **The clause's other sentences are the producer's, and they are checked rather than enforced.**
//! The key "shall match the values stored for both the F and UF keys", and the name has six
//! restrictions; an asset that breaks one is still listed and saved under the tree's key, with the
//! departure said in [`Assets::departures`], because withholding a file over its name would tell a
//! person less than handing it over with a sentence.

use pdf_syntax::{Dictionary, Document, Object, tree};

use super::{Attachment, MAX_ATTACHMENTS, read};

/// The assets of one rich media annotation, and what their names depart from.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    /// Each asset whose specification carries an embedded file, in the tree's order, named by its
    /// key.
    pub files: Vec<Attachment>,
    /// One sentence for each departure from §13.7.2.3.2's naming rules, naming the asset.
    pub departures: Vec<String>,
}

/// The six characters §13.7.2.3.2 excludes from an asset's name.
const EXCLUDED: [char; 6] = [':', '*', '"', '<', '>', '|'];

/// Table 333's `/RichMediaContent`, then Table 341's `/Assets`, of a `RichMedia` annotation.
///
/// Empty for an annotation of any other subtype, one with no content dictionary and one whose
/// content states no assets — Table 341 makes `/Assets` optional. A value that is not a file
/// specification carrying an embedded file is a name the tree holds and no file, and is skipped.
#[must_use]
pub fn assets(document: &Document, annotation: &Dictionary) -> Assets {
    let mut out = Assets::default();
    let subtype = document.get_key(annotation, "Subtype");
    if subtype.as_name().map(pdf_syntax::Name::as_bytes) != Some(b"RichMedia") {
        return out;
    }
    let content = document.get_key(annotation, "RichMediaContent");
    let Some(content) = content.as_dict() else {
        return out;
    };
    let tree = document.get_key(content, "Assets");
    let Some(tree) = tree.as_dict() else {
        return out;
    };
    for (key, value) in tree::name_pairs(tree, &|object| document.resolve(object)) {
        if out.files.len() >= MAX_ATTACHMENTS {
            break;
        }
        let resolved = document.resolve(&value);
        let Some(specification) = resolved.as_dict() else {
            continue;
        };
        let name = pdf_syntax::text_string(&key);
        let Some(file) = read(document, specification, name.clone()) else {
            continue;
        };
        out.departures
            .extend(departures(document, &name, specification));
        out.files.push(file);
    }
    out
}

/// What one asset's name departs from, by §13.7.2.3.2's rules in the clause's order.
///
/// > The text string that represents the file name in the name tree shall match the values stored
/// > for both the F and UF keys, which shall be the same value.
///
/// and then "\[t]he file name shall be encoded as a relative URI" with its restrictions: no
/// U+0000, between 1 and 255 characters, none of the six characters, and no FULL STOP last. The
/// relative URI is RFC 3986 section 4.2's `relative-ref`, whose characters are section 2's
/// unreserved and sub-delimiter characters, `:` `@` `/` `?` `#`, and a percent sign that starts a
/// two-digit escape; the excluded colon is the one that would make it a scheme.
fn departures(document: &Document, name: &str, specification: &Dictionary) -> Vec<String> {
    let mut out = Vec::new();
    let text = |key: &str| match document.get_key(specification, key) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    };
    let (file, unicode) = (text("F"), text("UF"));
    if file.as_deref() != Some(name) || unicode.as_deref() != Some(name) {
        out.push(format!(
            "the rich media asset {name:?} is filed under a name its file specification's /F \
             ({file:?}) and /UF ({unicode:?}) do not both state, which §13.7.2.3.2 requires"
        ));
    }
    let length = name.chars().count();
    let broken: Vec<&str> = [
        (name.contains('\0'), "it contains U+0000"),
        (
            !(1..=255).contains(&length),
            "it is not 1 to 255 characters long",
        ),
        (name.contains(EXCLUDED), "it contains one of : * \" < > |"),
        (name.ends_with('.'), "its last character is a FULL STOP"),
        (!is_relative_reference(name), "it is not a relative URI"),
    ]
    .into_iter()
    .filter_map(|(broken, why)| broken.then_some(why))
    .collect();
    if !broken.is_empty() {
        out.push(format!(
            "the rich media asset {name:?} breaks §13.7.2.3.2's naming rules: {}",
            broken.join("; ")
        ));
    }
    out
}

/// Whether every character of `name` is one RFC 3986's `relative-ref` admits, a `%` only as the
/// start of a two-digit escape.
fn is_relative_reference(name: &str) -> bool {
    let mut characters = name.chars();
    while let Some(character) = characters.next() {
        let admitted = character.is_ascii_alphanumeric()
            || "-._~!$&'()*+,;=:@/?#".contains(character)
            || (character == '%'
                && characters
                    .next()
                    .is_some_and(|digit| digit.is_ascii_hexdigit())
                && characters
                    .next()
                    .is_some_and(|digit| digit.is_ascii_hexdigit()));
        if !admitted {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::assets;
    use pdf_syntax::{Document, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// The annotation, object 2.
    fn annotation(doc: &Document) -> pdf_syntax::Dictionary {
        doc.get(ObjectId::new(2, 0))
            .as_dict()
            .cloned()
            .expect("the annotation is a dictionary")
    }

    /// §13.7.2.3.2's EXAMPLE, its `/F` and `/UF` both the key: one asset, filed under `3D.prc`,
    /// with nothing to say of its name.
    #[test]
    fn the_clauses_example_is_one_asset_with_no_departure() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 1 1] /RichMediaContent << /Assets 3 0 \
             R >> >>",
            "<< /Names [(3D.prc) 4 0 R] >>",
            "<< /Type /Filespec /F (3D.prc) /UF (3D.prc) /EF << /F 5 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 3 >>\nstream\nprc\nendstream",
        ]);
        let found = assets(&doc, &annotation(&doc));
        let names: Vec<&str> = found.files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["3D.prc"]);
        assert!(found.departures.is_empty(), "{:?}", found.departures);
    }

    /// A key its `/UF` does not state, and a name that breaks three restrictions, are listed all
    /// the same and said; a value with no `/EF` is a name and no file.
    #[test]
    fn a_departing_name_is_listed_and_said_and_a_file_outside_is_skipped() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 1 1] /RichMediaContent << /Assets << \
             /Names [(a.swf) 3 0 R (b|c d.) 4 0 R (outside.flv) 6 0 R] >> >> >>",
            "<< /Type /Filespec /F (a.swf) /UF (other.swf) /EF << /F 5 0 R >> >>",
            "<< /Type /Filespec /F (b|c d.) /UF (b|c d.) /EF << /F 5 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 3 >>\nstream\nswf\nendstream",
            "<< /Type /Filespec /F (outside.flv) /UF (outside.flv) >>",
        ]);
        let found = assets(&doc, &annotation(&doc));
        let names: Vec<&str> = found.files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["a.swf", "b|c d."]);
        assert_eq!(found.departures.len(), 2, "{:?}", found.departures);
        assert!(found.departures[0].contains("\"other.swf\""));
        assert!(
            found.departures[1].contains(
                "one of : * \" < > |; its last character is a FULL \
                 STOP; it is not a relative URI"
            ),
            "{}",
            found.departures[1]
        );
    }

    /// §13.7.2.3.1's Table 341, reached through an indirect `/RichMediaContent`: its `/Assets` is
    /// "[a] name tree of embedded file specification dictionaries" and is read, while its
    /// `/Configurations` and `/Views` are the artwork's and give no file; a content dictionary
    /// stating no `/Assets` has none, because the table makes the entry optional (ADR 2410).
    #[test]
    fn table_341s_assets_are_read_and_its_configurations_and_views_give_no_file() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 1 1] /RichMediaContent 3 0 R >>",
            "<< /Type /RichMediaContent /Assets << /Names [(clip.mp4) 4 0 R] >> \
             /Configurations [6 0 R] /Views [7 0 R] >>",
            "<< /Type /Filespec /F (clip.mp4) /UF (clip.mp4) /EF << /F 5 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 3 >>\nstream\nmp4\nendstream",
            "<< /Type /RichMediaConfiguration /Subtype /Video /Instances [8 0 R] >>",
            "<< /Type /3DView /XN (Default) >>",
            "<< /Type /RichMediaInstance /Subtype /Video /Asset 4 0 R >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 1 1] /RichMediaContent \
             << /Configurations [6 0 R] >> >>",
        ]);
        let found = assets(&doc, &annotation(&doc));
        let names: Vec<&str> = found.files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["clip.mp4"]);
        let without = doc
            .get(ObjectId::new(9, 0))
            .as_dict()
            .cloned()
            .expect("the second annotation is a dictionary");
        assert!(assets(&doc, &without).files.is_empty());
    }

    /// Only a `RichMedia` annotation has assets: the same content dictionary on a `Screen`
    /// annotation is not Table 333's.
    #[test]
    fn another_subtype_has_no_assets() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /Screen /Rect [0 0 1 1] /RichMediaContent << /Assets << \
             /Names [(a.swf) 3 0 R] >> >> >>",
            "<< /Type /Filespec /F (a.swf) /UF (a.swf) /EF << /F 4 0 R >> >>",
            "<< /Type /EmbeddedFile /Length 3 >>\nstream\nswf\nendstream",
        ]);
        assert!(assets(&doc, &annotation(&doc)).files.is_empty());
    }
}
