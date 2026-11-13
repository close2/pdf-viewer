//! ISO 32000-2 §9.9.2 for the appearances a save carries whole from an FDF file.
//!
//! §12.7.8.3.2 replaces a field's `/AP` with the FDF field's, and §12.7.8.3.4 adds annotations
//! that carry their own; both are the FDF producer's streams, written into this file as they came
//! (ADR 1223). Their fonts are subsets the producer named, and §9.9.2's "different subsets of the
//! same font in the same PDF file shall have different tags" reaches them once they are in the
//! file: a producer's `ABCDEF+Helvetica` may meet a different subset under the same name that the
//! target file already states, one this save constructs, or one another imported appearance
//! carries. So each is named through the save's one [`SubsetNames`] (ADR 2832) and, where its name
//! is taken, given the next tag — in its dictionaries and in its program (ADR 2862). The marks are
//! untouched: a tag is a name, and the glyphs the stream shows are the producer's.

use pdf_syntax::{Dictionary, Document, Object};

use super::SubsetNames;

/// How deep [`SubsetNames::name_imported`] follows an appearance's direct objects: an
/// appearance dictionary, a state's subdictionary, a stream, its resources, a form within it and
/// its resources, a Type 3 font's own resources — a bound against a hostile nesting, well past
/// what an appearance holds.
const IMPORTED_DEPTH: usize = 24;

impl SubsetNames {
    /// `appearance`, an `/AP` an FDF file carried, with every font a resource dictionary in it
    /// holds named through this record.
    ///
    /// A font is reached wherever a `/Font` resource dictionary is — the appearance stream's,
    /// a form's it paints, a Type 3 font's glyph procedures' — because each is a subset written
    /// into the file. A font held by reference is the target file's own and already among the
    /// names it states, so only fonts held directly are named.
    #[must_use]
    pub(crate) fn name_imported(&mut self, document: &Document, appearance: Object) -> Object {
        self.imported(document, appearance, IMPORTED_DEPTH)
    }

    fn imported(&mut self, document: &Document, object: Object, depth: usize) -> Object {
        let Some(depth) = depth.checked_sub(1) else {
            return object;
        };
        match object {
            Object::Dictionary(dict) => {
                Object::Dictionary(self.imported_dict(document, &dict, depth))
            }
            Object::Stream(stream) => {
                let mut stream = (*stream).clone();
                stream.dict = self.imported_dict(document, &stream.dict, depth);
                Object::Stream(std::sync::Arc::new(stream))
            }
            Object::Array(items) => Object::Array(
                items
                    .into_iter()
                    .map(|item| self.imported(document, item, depth))
                    .collect(),
            ),
            other => other,
        }
    }

    fn imported_dict(
        &mut self,
        document: &Document,
        dict: &Dictionary,
        depth: usize,
    ) -> Dictionary {
        let mut out = dict.clone();
        for (key, value) in dict.iter() {
            let named = match (key.as_bytes(), value) {
                (b"Font", Object::Dictionary(fonts)) => {
                    let mut named = fonts.clone();
                    for (font_key, font) in fonts.iter() {
                        let Object::Dictionary(font) = font else {
                            continue;
                        };
                        let mut font = self.imported_dict(document, font, depth);
                        self.name(&mut font, Some(document));
                        named.insert(font_key.clone(), Object::Dictionary(font));
                    }
                    Object::Dictionary(named)
                }
                _ => self.imported(document, value.clone(), depth),
            };
            out.insert(key.clone(), named);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SubsetNames, for_a_file_in, is_subset_tagged, stream};
    use pdf_syntax::{Dictionary, Document, Name, Object};
    use std::fmt::Write as _;

    /// A compact `CIDFont` program as far as its Name INDEX (Adobe Technical Note #5176's
    /// four-byte header, a count of one, a one-byte offset size, two offsets) naming `name`.
    fn compact(name: &[u8]) -> Vec<u8> {
        let mut out = vec![1, 0, 4, 1, 0, 1, 1, 1];
        out.push(u8::try_from(name.len().saturating_add(1)).expect("short"));
        out.extend_from_slice(name);
        out.extend_from_slice(b"glyphs");
        out
    }

    fn name(value: &[u8]) -> Object {
        Object::Name(Name::new(value.to_vec()))
    }

    /// An FDF producer's Type 0 font over a compact `CIDFont` subset named `subset`, holding
    /// `glyphs` after its Name INDEX, as Table 117 and Table 119 lay it out — every object
    /// direct, as an imported appearance carries it.
    fn producers_font(subset: &[u8], glyphs: &[u8]) -> Dictionary {
        let mut program = compact(subset);
        program.extend_from_slice(glyphs);
        let mut file = Dictionary::new();
        file.insert(Name::new(b"Subtype".to_vec()), name(b"CIDFontType0C"));
        let mut descriptor = Dictionary::new();
        descriptor.insert(Name::new(b"FontName".to_vec()), name(subset));
        descriptor.insert(
            Name::new(b"FontFile3".to_vec()),
            stream(file, program.into()),
        );
        let mut descendant = Dictionary::new();
        descendant.insert(Name::new(b"Subtype".to_vec()), name(b"CIDFontType0"));
        descendant.insert(Name::new(b"BaseFont".to_vec()), name(subset));
        descendant.insert(
            Name::new(b"FontDescriptor".to_vec()),
            Object::Dictionary(descriptor),
        );
        let mut font = Dictionary::new();
        font.insert(Name::new(b"Subtype".to_vec()), name(b"Type0"));
        let mut combined = subset.to_vec();
        combined.extend_from_slice(b"-Identity-H");
        font.insert(Name::new(b"BaseFont".to_vec()), name(&combined));
        font.insert(
            Name::new(b"DescendantFonts".to_vec()),
            Object::Array(vec![Object::Dictionary(descendant)]),
        );
        font
    }

    /// An appearance dictionary whose normal appearance paints with `font` as `/F1`.
    fn appearance(font: Dictionary) -> Object {
        let mut fonts = Dictionary::new();
        fonts.insert(Name::new(b"F1".to_vec()), Object::Dictionary(font));
        let mut resources = Dictionary::new();
        resources.insert(Name::new(b"Font".to_vec()), Object::Dictionary(fonts));
        let mut form = Dictionary::new();
        form.insert(
            Name::new(b"Resources".to_vec()),
            Object::Dictionary(resources),
        );
        let mut ap = Dictionary::new();
        ap.insert(
            Name::new(b"N".to_vec()),
            stream(form, b"BT /F1 12 Tf <0001> Tj ET".to_vec().into()),
        );
        Object::Dictionary(ap)
    }

    /// The `/F1` font of an appearance [`appearance`] built.
    fn font_of(appearance: &Object) -> Dictionary {
        let Object::Dictionary(ap) = appearance else {
            panic!("an appearance dictionary");
        };
        let Some(Object::Stream(normal)) = ap.get("N") else {
            panic!("a normal appearance");
        };
        normal
            .dict
            .get("Resources")
            .and_then(Object::as_dict)
            .and_then(|resources| resources.get("Font"))
            .and_then(Object::as_dict)
            .and_then(|fonts| fonts.get("F1"))
            .and_then(Object::as_dict)
            .cloned()
            .expect("the font")
    }

    /// Every name the font states, in the order Table 119, Table 117 and Table 120 give them:
    /// the Type 0 font's `/BaseFont`, the `CIDFont`'s, the descriptor's `/FontName`, and the
    /// program's Name INDEX entry.
    fn names_of(font: &Dictionary) -> [Vec<u8>; 4] {
        let base = |dict: &Dictionary| {
            dict.get("BaseFont")
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec())
                .unwrap_or_default()
        };
        let descendant = font
            .get("DescendantFonts")
            .and_then(Object::as_array)
            .and_then(|items| items.first())
            .and_then(Object::as_dict)
            .expect("a CIDFont");
        let descriptor = descendant
            .get("FontDescriptor")
            .and_then(Object::as_dict)
            .expect("a descriptor");
        let Some(Object::Stream(program)) = descriptor.get("FontFile3") else {
            panic!("a program");
        };
        let program = super::super::decoded_program(program).expect("decodes");
        // The second offset is one past the name's end, the first being 1.
        let length = usize::from(program[8]).saturating_sub(1);
        [
            base(font),
            base(descendant),
            descriptor
                .get("FontName")
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec())
                .unwrap_or_default(),
            program[9..length.saturating_add(9)].to_vec(),
        ]
    }

    /// A file whose one object is a `CIDFont` named `name`, as the target of an import states it.
    fn a_file_stating(name: &[u8]) -> Document {
        let name = String::from_utf8(name.to_vec()).expect("ASCII");
        let object = format!(
            "1 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{name} >>\nendobj\n"
        );
        let mut out = String::from("%PDF-1.7\n");
        let offset = out.len();
        out.push_str(&object);
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 2\n0000000000 65535 f \n{offset:010} 00000 n \ntrailer\n<< /Size 2 >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// §9.9.2: "different subsets of the same font in the same PDF file shall have different
    /// tags". The target file states `EOODIA+Poetica` — the clause's own EXAMPLE name — and an
    /// FDF carries a different subset under it, so the imported one takes the next tag, and
    /// takes it everywhere the name is stated: Table 119's `/BaseFont` keeps its `CMap` suffix,
    /// and Table 117's "shall be the value of the `CIDFontName` entry in the `CIDFont` program"
    /// holds because the Name INDEX moves with the dictionaries.
    #[test]
    fn an_imported_subset_passes_over_a_name_the_file_states() {
        let document = a_file_stating(b"EOODIA+Poetica");
        let imported = SubsetNames::default().name_imported(
            &document,
            appearance(producers_font(b"EOODIA+Poetica", b"a")),
        );
        assert_eq!(
            names_of(&font_of(&imported)),
            [
                b"EOODIB+Poetica-Identity-H".to_vec(),
                b"EOODIB+Poetica".to_vec(),
                b"EOODIB+Poetica".to_vec(),
                b"EOODIB+Poetica".to_vec(),
            ]
        );
        let control = SubsetNames::default().name_imported(
            &Document::empty(),
            appearance(producers_font(b"EOODIA+Poetica", b"a")),
        );
        assert_eq!(
            names_of(&font_of(&control))[1],
            b"EOODIA+Poetica".to_vec(),
            "the control: a file that states no such name leaves the producer's"
        );
    }

    /// Two imported appearances carrying two different subsets under one tag are given two, and
    /// the same subset carried twice keeps one name — §9.9.2 forbids one tag for two subsets and
    /// asks nothing of one subset written twice.
    #[test]
    fn two_imported_subsets_under_one_tag_are_given_two() {
        let document = Document::empty();
        let mut names = SubsetNames::default();
        let first = names.name_imported(
            &document,
            appearance(producers_font(b"EOODIA+Poetica", b"a")),
        );
        let again = names.name_imported(
            &document,
            appearance(producers_font(b"EOODIA+Poetica", b"a")),
        );
        let other = names.name_imported(
            &document,
            appearance(producers_font(b"EOODIA+Poetica", b"b")),
        );
        let (first, again, other) = (
            names_of(&font_of(&first)),
            names_of(&font_of(&again)),
            names_of(&font_of(&other)),
        );
        assert_eq!(first, again);
        assert_ne!(first[1], other[1]);
        assert!(other.iter().skip(1).all(|name| name == &other[1]));
        assert!(is_subset_tagged(&other[1]));
    }

    /// A subset this save constructs and one an FDF carries are named through the same record,
    /// so an imported subset under the name a constructed one took passes over it.
    #[test]
    fn an_imported_subset_passes_over_a_name_this_save_wrote() {
        let program: std::sync::Arc<[u8]> = std::sync::Arc::from(
            include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf").as_slice(),
        );
        let glyphs = [('G', "G".to_owned())].into_iter().collect();
        let machine = super::super::machine_font(&program, &glyphs)
            .expect("the bundled face covers G")
            .0;
        let mut fonts = Dictionary::new();
        fonts.insert(Name::new(b"F1".to_vec()), Object::Dictionary(machine));
        let mut resources = Dictionary::new();
        resources.insert(Name::new(b"Font".to_vec()), Object::Dictionary(fonts));
        let document = Document::empty();
        let mut names = SubsetNames::default();
        let constructed = for_a_file_in(&document, &resources, &mut names).expect("embedded");
        let taken = constructed
            .get("Font")
            .and_then(Object::as_dict)
            .and_then(|fonts| fonts.get("F1"))
            .and_then(Object::as_dict)
            .and_then(|font| font.get("DescendantFonts"))
            .and_then(Object::as_array)
            .and_then(|items| items.first())
            .and_then(Object::as_dict)
            .and_then(|descendant| descendant.get("BaseFont"))
            .and_then(Object::as_name)
            .expect("a CIDFont name")
            .as_bytes()
            .to_vec();
        assert!(is_subset_tagged(&taken));
        let imported = names.name_imported(&document, appearance(producers_font(&taken, b"a")));
        let moved = names_of(&font_of(&imported));
        assert_ne!(moved[1], taken);
        assert_eq!(moved[1].get(6..), taken.get(6..), "the face's name is kept");
        let control = SubsetNames::default()
            .name_imported(&document, appearance(producers_font(&taken, b"a")));
        assert_eq!(
            names_of(&font_of(&control))[1],
            taken,
            "the control: a record of its own"
        );
    }
}
