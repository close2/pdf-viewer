//! Every resource dictionary the content streams beneath a page or a group can name — ISO 32000-2
//! §7.8.2 and §7.8.3 — for the readers that need an answer *before* the streams are run.
//!
//! # Why one walk
//!
//! §7.8.2 lists the content streams a page's marks come from: the page's own, and "forms (see
//! 8.10, "Form XObjects"), patterns (8.7, "Patterns"), certain fonts (9.6.4, "Type 3 fonts"), and
//! annotation appearances (12.5.5, "Appearance streams")". §7.8.3 gives each of them a resource
//! dictionary, and Errata Collection 3's Issue #128 restates where a Type 3 glyph's is found: the
//! glyph description's own `/Resources` first, then its font's, then the page's. The interpreter
//! takes every one of those routes as it draws. Three readers ask a question of the same
//! dictionaries ahead of it — which optional content groups a page can mark
//! (`optional_content::groups_referenced_by`), which spot colourants it can paint
//! (`colourants::spot_colourants`), and whether a soft mask's group can reach a blend mode
//! (`content::resources_blend`) — and each once walked them itself and each missed a different
//! route (review 2234's B01 and B02). [`ResourceWalk`] is the list written once, so a route is
//! taken by all three or by none. ADR 3645.
//!
//! # What is walked
//!
//! From a resource dictionary: each `/XObject` that is a form (§8.10, Table 93's `/Resources`),
//! each `/Pattern` that states resources (§8.7.3's tiling pattern, Table 74), and each Type 3
//! `/Font` (§9.6.4) — every `/CharProcs` glyph stream's own `/Resources` and then the font's,
//! Issue #128's order. From a page, optionally, each annotation's `/AP` `/N`: a stream, or a
//! dictionary of them keyed by appearance state (§12.5.5).
//!
//! **A soft mask's group is not walked.** It is reached through `/ExtGState`, not as a mark of
//! the page: §11.7.3 says spot colours "shall not be available in a transparency group XObject
//! that is used to define a soft mask", and the blend question is asked of each mask's group
//! by itself. A reader that wants a mask's group starts a walk at it.
//!
//! The walk reads what the dictionaries *state*, not what the streams *use*: it does not parse a
//! stream to learn which names its operators select. Every caller's answer is an upper bound for
//! that reason, and each says why the over-reading is the safe direction for it.
//!
//! # The bounds
//!
//! The nesting is the document's word, so the walk is finite twice over (principle 3): every
//! indirect object is entered at most once — a form, a pattern, a font, a glyph, an appearance and
//! a shared resource dictionary alike — which ends a cycle; and no dictionary deeper than the
//! interpreter's own form bound is read, since content nested past it is never drawn
//! ([`crate::content::MAX_FORM_DEPTH`]). Where that depth cut something,
//! [`ResourceWalk::truncated`] says so. The walk runs from an explicit stack, so a deep chain costs
//! heap rather than the thread's stack.
#![expect(
    clippy::doc_markdown,
    reason = "the module quotes §7.8.2 and §11.7.3 verbatim, and a backtick inside a quotation \
              would make it something other than the standard's words"
)]

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// One object the walk has still to read, and what it is being read as.
enum Pending {
    /// A resource dictionary (§7.8.3, Table 34).
    Resources(Object),
    /// An `/XObject` entry: a form's resources are followed, an image has none (§8.10).
    XObject(Object),
    /// A `/Pattern` entry: a tiling pattern's resources (§8.7.3); a shading pattern states none.
    Pattern(Object),
    /// A `/Font` entry: a Type 3 font's glyphs and its own resources (§9.6.4, Issue #128).
    Font(Object),
    /// A content stream drawn as itself — a Type 3 glyph description — whose own `/Resources`
    /// is followed.
    Stream(Object),
    /// An annotation's normal appearance: a stream, or a dictionary of them (§12.5.5).
    Appearance(Object),
}

/// Whether a page's walk includes its annotations' normal appearances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Annotations {
    /// Each annotation's `/AP` `/N`, after the page's own resources, as §12.5.5 draws them over
    /// the page's content.
    Appearances,
    /// The page's content only.
    Excluded,
}

/// A depth-first walk over every resource dictionary beneath a root, in document order: a
/// dictionary is yielded before anything beneath it, and what it names is read in the order
/// `/Pattern`, `/XObject`, `/Font`, each subdictionary's own key order within that. The same
/// document therefore always gives the same sequence.
pub(crate) struct ResourceWalk<'a> {
    document: &'a Document,
    /// The objects still to read, each with the depth of the resources it leads to; the last is
    /// read next.
    stack: Vec<(Pending, usize)>,
    /// Every indirect object already entered, whatever it was read as. One set serves every
    /// kind because each kind contributes the same thing — its `/Resources`, and a Type 3 font
    /// its glyphs' — however it was reached.
    entered: BTreeSet<ObjectId>,
    truncated: bool,
}

impl<'a> ResourceWalk<'a> {
    /// The walk beneath `resources`, which is yielded first: a soft mask's group, or a page's
    /// resources without its annotations.
    pub(crate) fn new(document: &'a Document, resources: &Dictionary) -> Self {
        Self {
            document,
            stack: vec![(Pending::Resources(Object::Dictionary(resources.clone())), 0)],
            entered: BTreeSet::new(),
            truncated: false,
        }
    }

    /// The walk beneath `page`'s resources, and, if asked, beneath each of its annotations'
    /// normal appearances after them.
    pub(crate) fn of_page(
        document: &'a Document,
        page: &crate::page::Page,
        annotations: Annotations,
    ) -> Self {
        let mut walk = Self::new(document, &page.resources);
        if annotations == Annotations::Appearances {
            let mut appearances = Vec::new();
            if let Some(list) = document.get_key(&page.dict, "Annots").as_array() {
                for annotation in list {
                    let annotation = document.resolve(annotation);
                    if let Some(annotation) = annotation.as_dict()
                        && let Some(states) = document.get_key(annotation, "AP").as_dict()
                        && let Some(normal) = states.get("N")
                    {
                        appearances.push((Pending::Appearance(normal.clone()), 1));
                    }
                }
            }
            // Beneath the page's own resources on the stack, so they come after its subtree.
            walk.stack.splice(0..0, appearances.into_iter().rev());
        }
        walk
    }

    /// Whether a dictionary was left unread because it was nested past
    /// [`crate::content::MAX_FORM_DEPTH`].
    pub(crate) fn truncated(&self) -> bool {
        self.truncated
    }

    /// Stacks `dict`'s `/Resources`, unresolved so that its reference is seen, at `depth`.
    fn push_resources(&mut self, dict: &Dictionary, depth: usize) {
        if let Some(resources) = dict.get("Resources") {
            self.stack
                .push((Pending::Resources(resources.clone()), depth));
        }
    }

    /// Stacks what one resource dictionary at `depth` names, so that it comes off in the order
    /// the dictionary states it.
    fn push_children(&mut self, resources: &Dictionary, depth: usize) {
        let below = depth.saturating_add(1);
        let mut children = Vec::new();
        for (category, pending) in [
            ("Pattern", Pending::Pattern as fn(Object) -> Pending),
            ("XObject", Pending::XObject),
            ("Font", Pending::Font),
        ] {
            if let Some(entries) = self.document.get_key(resources, category).as_dict() {
                children.extend(
                    entries
                        .iter()
                        .map(|(_, value)| (pending(value.clone()), below)),
                );
            }
        }
        self.stack.extend(children.into_iter().rev());
    }

    /// Reads one pending object, returning the resource dictionary it is, if it is one.
    fn read(&mut self, pending: &Pending, depth: usize) -> Option<Dictionary> {
        let object = match pending {
            Pending::Resources(object)
            | Pending::XObject(object)
            | Pending::Pattern(object)
            | Pending::Font(object)
            | Pending::Stream(object)
            | Pending::Appearance(object) => object,
        };
        if let Some(id) = object.as_reference()
            && !self.entered.insert(id)
        {
            return None;
        }
        let object = self.document.resolve(object);
        match pending {
            Pending::Resources(_) => {
                let resources = object.as_dict()?;
                if depth > crate::content::MAX_FORM_DEPTH {
                    self.truncated = true;
                    return None;
                }
                self.push_children(resources, depth);
                return Some(resources.clone());
            }
            Pending::XObject(_) => {
                let form = object.as_stream()?;
                if self
                    .document
                    .get_key(&form.dict, "Subtype")
                    .as_name()
                    .is_some_and(|subtype| subtype.as_bytes() == b"Form")
                {
                    self.push_resources(&form.dict, depth);
                }
            }
            Pending::Pattern(_) => {
                // §8.7.3's tiling pattern is a stream with resources; §8.7.4's shading pattern is
                // a dictionary without them, so reading the key where it is present decides both.
                if let Some(pattern) = object.as_dict() {
                    self.push_resources(pattern, depth);
                }
            }
            Pending::Font(_) => {
                let font = object.as_dict()?;
                let subtype = self.document.get_key(font, "Subtype");
                if subtype.as_name().map(pdf_syntax::Name::as_bytes) != Some(b"Type3".as_slice()) {
                    return None;
                }
                // Issue #128's search order: a glyph's own resources, then its font's. Pushed in
                // reverse, so the glyphs come off first and in the order `/CharProcs` states them.
                self.push_resources(font, depth);
                if let Some(glyphs) = self.document.get_key(font, "CharProcs").as_dict() {
                    let glyphs: Vec<_> = glyphs
                        .iter()
                        .map(|(_, glyph)| (Pending::Stream(glyph.clone()), depth))
                        .collect();
                    self.stack.extend(glyphs.into_iter().rev());
                }
            }
            Pending::Stream(_) => {
                if let Some(stream) = object.as_stream() {
                    self.push_resources(&stream.dict, depth);
                }
            }
            Pending::Appearance(_) => match &object {
                Object::Stream(stream) => self.push_resources(&stream.dict, depth),
                Object::Dictionary(states) => {
                    let states: Vec<_> = states
                        .iter()
                        .map(|(_, state)| (Pending::Appearance(state.clone()), depth))
                        .collect();
                    self.stack.extend(states.into_iter().rev());
                }
                _ => {}
            },
        }
        None
    }
}

impl Iterator for ResourceWalk<'_> {
    type Item = Dictionary;

    fn next(&mut self) -> Option<Dictionary> {
        while let Some((pending, depth)) = self.stack.pop() {
            if let Some(resources) = self.read(&pending, depth) {
                return Some(resources);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use pdf_syntax::{Dictionary, Document};

    use super::{Annotations, ResourceWalk};

    /// A one-page file: object 3 is the page, whose `/Resources` and `/Annots` are `page`'s
    /// text, and `objects` are numbered from 5.
    fn file(page: &str, objects: &[String]) -> Vec<u8> {
        let mut all = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] {page} /Contents 4 0 R >>"),
            "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        ];
        all.extend(objects.iter().cloned());
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in all.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        out.into_bytes()
    }

    /// A form `XObject` with `resources` as its `/Resources`.
    fn form(resources: &str) -> String {
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Resources {resources} /Length 0 \
             >>\nstream\n\nendstream"
        )
    }

    /// The `/Tag` entry of each yielded dictionary, in the walk's order: every test dictionary
    /// carries one naming itself, a key Table 34 does not define and the walk does not read.
    fn tags(walk: ResourceWalk<'_>) -> Vec<String> {
        walk.map(|resources: Dictionary| {
            resources
                .get("Tag")
                .and_then(|tag| tag.as_name())
                .map_or_else(String::new, |tag| {
                    String::from_utf8_lossy(tag.as_bytes()).into_owned()
                })
        })
        .collect()
    }

    fn walk_of(page: &str, objects: &[String], annotations: Annotations) -> Vec<String> {
        let document = Document::open(file(page, objects)).expect("the fixture is a valid PDF");
        let page = crate::Pages::new(&document).get(0).expect("page one");
        tags(ResourceWalk::of_page(&document, &page, annotations))
    }

    /// §7.8.3 as Errata Collection 3's Issue #128 restates it: a glyph description's resources
    /// are its own `/Resources` first, then its Type 3 font's. Both are walked, the glyphs first
    /// in `/CharProcs` order, and a font of another subtype states none.
    #[test]
    fn a_type3_glyphs_own_resources_come_before_its_fonts() {
        let glyph = |tag: &str| {
            format!("<< /Resources << /Tag /{tag} >> /Length 0 >>\nstream\n\nendstream")
        };
        let order = walk_of(
            "/Resources << /Tag /Page /Font << /F1 5 0 R /F2 8 0 R >> >>",
            &[
                "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [1 0 0 1 0 0] \
                 /CharProcs << /a 6 0 R /b 7 0 R >> /Resources << /Tag /Font >> >>"
                    .to_owned(),
                glyph("GlyphA"),
                glyph("GlyphB"),
                "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Resources << /Tag /Not >> >>"
                    .to_owned(),
            ],
            Annotations::Excluded,
        );
        assert_eq!(order, ["Page", "GlyphA", "GlyphB", "Font"]);
    }

    /// §7.8.2's other three kinds: a tiling pattern's cell, a form, and an annotation's normal
    /// appearance — a dictionary of appearance states here — which comes after the page's own
    /// subtree and only where it is asked for.
    #[test]
    fn patterns_forms_and_appearances_are_walked() {
        let page = "/Resources << /Tag /Page /Pattern << /P0 5 0 R >> /XObject << /X0 6 0 R >> >> \
                    /Annots [7 0 R]";
        let objects = [
            "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 1 1] /XStep 1 /YStep 1 \
             /Resources << /Tag /Cell >> /Length 0 >>\nstream\n\nendstream"
                .to_owned(),
            form("<< /Tag /Form >>"),
            "<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] /AP << /N << /On 8 0 R >> >> >>"
                .to_owned(),
            form("<< /Tag /Appearance >>"),
        ];
        assert_eq!(
            walk_of(page, &objects, Annotations::Appearances),
            ["Page", "Cell", "Form", "Appearance"]
        );
        assert_eq!(
            walk_of(page, &objects, Annotations::Excluded),
            ["Page", "Cell", "Form"]
        );
    }

    /// A form naming itself, and two forms sharing one resource dictionary: each indirect object
    /// is entered once, so the cycle ends and the shared dictionary is read once.
    #[test]
    fn a_cycle_ends_and_a_shared_dictionary_is_read_once() {
        let order = walk_of(
            "/Resources << /Tag /Page /XObject << /A 5 0 R /B 6 0 R >> >>",
            &[
                form("<< /Tag /Self /XObject << /Again 5 0 R >> >>"),
                form("7 0 R"),
                "<< /Tag /Shared /XObject << /C 8 0 R >> >>".to_owned(),
                form("7 0 R"),
            ],
            Annotations::Excluded,
        );
        assert_eq!(order, ["Page", "Self", "Shared"]);
    }

    /// A chain of distinct forms deeper than the interpreter draws is cut at its bound, and the
    /// walk says so; one inside the bound is not.
    #[test]
    fn a_chain_past_the_interpreters_bound_is_cut_and_said() {
        let chain = |length: usize| {
            let objects: Vec<String> = (0..length)
                .map(|index| {
                    let next = index.saturating_add(6);
                    if index.saturating_add(1) == length {
                        form("<< >>")
                    } else {
                        form(&format!("<< /XObject << /X {next} 0 R >> >>"))
                    }
                })
                .collect();
            let document =
                Document::open(file("/Resources << /XObject << /X 5 0 R >> >>", &objects))
                    .expect("the fixture is a valid PDF");
            let page = crate::Pages::new(&document).get(0).expect("page one");
            let mut walk = ResourceWalk::of_page(&document, &page, Annotations::Excluded);
            let read = walk.by_ref().count();
            (read, walk.truncated())
        };
        let bound = crate::content::MAX_FORM_DEPTH;
        assert_eq!(chain(bound), (bound.saturating_add(1), false));
        assert_eq!(
            chain(bound.saturating_add(2)),
            (bound.saturating_add(1), true)
        );
    }

    /// §11.6.3's blend mode stated in a Type 3 glyph description's own `/Resources` (review
    /// 2234's B01): `resources_blend` reads it through the walk; the same glyph without it reads
    /// `Normal`.
    #[test]
    fn a_blend_mode_in_a_glyphs_own_resources_is_seen() {
        let blends = |mode: &str| {
            let document = Document::open(file(
                "/Resources << /Font << /F1 5 0 R >> >>",
                &[
                    "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] \
                     /FontMatrix [1 0 0 1 0 0] /CharProcs << /a 6 0 R >> >>"
                        .to_owned(),
                    format!(
                        "<< /Resources << /ExtGState << /G0 << /BM /{mode} >> >> >> /Length 0 \
                         >>\nstream\n\nendstream"
                    ),
                ],
            ))
            .expect("the fixture is a valid PDF");
            let page = crate::Pages::new(&document).get(0).expect("page one");
            crate::content::resources_blend(&document, &page.resources)
        };
        assert!(blends("Multiply"));
        assert!(!blends("Normal"));
    }
}
