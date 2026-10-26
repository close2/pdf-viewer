//! Table 50's marked-content operators, §14.6, and what the sections they open and close record.

use super::machine::Machine;
use super::operands::name_at;

use pdf_syntax::{Dictionary, Name, Object};

use crate::content::marked::Marked;

use crate::content::reader::Operator;
use crate::content::report::{ArtifactSource, ArtifactSpan, MarkedSpan};

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// The marked-content operators of Table 50 (ISO 32000-2 §14.6): `MP`, `DP`, `BMC`, `BDC` and
    /// `EMC`.
    pub(super) fn run_marked_content_op(
        &mut self,
        operator: Operator,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        let Machine {
            marked,
            replaced_ends_at,
            ..
        } = machine;
        match operator {
            // §8.11.3.2: a marked-content section is optional content when its tag is
            // `OC` and its property list names a group or a membership dictionary.
            // Because a group is an indirect object, the operand is a *name* into the
            // resource dictionary's `/Properties`; an inline dictionary cannot carry
            // one, so it governs nothing.
            Operator::BeginMarkedProperties => {
                self.begin_marked_properties(operands, resources, marked);
            }
            Operator::BeginMarked => {
                let tag = name_at(operands, 0);
                let tag = tag.as_ref().map_or(&[][..], Name::as_bytes);
                // The generic forms: `/Artifact BMC` states an artifact with no property
                // list, and `/ReversedChars BMC` is the form §14.8.2.5.3's own EXAMPLE uses.
                let reversed = tag == b"ReversedChars";
                marked.push(Marked {
                    starts_at: self.text.len(),
                    artifact: (tag == b"Artifact").then(crate::structure::Artifact::default),
                    reversed,
                    ..Marked::default()
                });
                if reversed {
                    self.reversed_chars = self.reversed_chars.saturating_add(1);
                }
            }
            Operator::EndMarked => self.end_marked_content(marked, replaced_ends_at),
            // `MP` and `DP` mark a point in the stream and carry structure rather than drawing.
            _ => {}
        }
    }

    /// `BDC`: opens a marked-content section whose property list may govern optional content
    /// (§8.11.3.2), name an artifact (§14.8.2.2.2), reverse its characters (§14.8.2.5.3), carry
    /// replacement text and descriptions (§14.9), an identifier (§14.7.5.2) or associated files
    /// (§14.13.5). One property list is read for all of them (ADR 2320).
    fn begin_marked_properties(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        marked: &mut Vec<Marked>,
    ) {
        // A *tag* is one of the standard's own names rather than a resource name, so
        // it is compared against an ASCII literal — as bytes, which §7.3.5 makes the
        // comparison a name has ("an exact binary match") and which costs nothing.
        let tag = name_at(operands, 0);
        let tag = tag.as_ref().map_or(&[][..], Name::as_bytes);
        let hides = tag == b"OC"
            && name_at(operands, 1).is_some_and(|name| {
                self.unresolved_resource(resources, "Properties", &name)
                    .is_some_and(|oc| !self.shows_optional_content(&oc))
            });
        // §14.9's four entries, all from the one property list this section
        // names, so that a tagged page reads it once rather than four times.
        let (actual_text, described) = self.accessibility(resources, operands.get(1));
        // §14.8.2.2.2's two forms of an artifact are `/Artifact BMC` and
        // `/Artifact <<propertyList>> BDC`; this is the second, and the property
        // list is Table 363's.
        let artifact = (tag == b"Artifact").then(|| {
            self.property_list(resources, operands.get(1))
                .map(|list| crate::structure::Artifact::read(self.document, &list))
                .unwrap_or_default()
        });
        let reversed = tag == b"ReversedChars";
        // §14.13.5: "One or more files may be associated with sections of content in
        // a content stream by enclosing those sections between the marked-content
        // operators BDC and EMC … with a marked-content tag of AF." NOTE 2 is why
        // this is on `BDC` alone: "[t]he BMC operator does not take properties and
        // therefore cannot be used with the AF key." The *tag* is `AF`; the key
        // inside the property list is `/MCAF` since Errata Collection 3, and the
        // two are not the same word by accident — see
        // `attachment::associated_in_property_list`. A named resource that is the
        // array itself is the clause's own example's form (ADR 1461).
        let associated = if tag == b"AF" {
            self.associated_files(resources, operands.get(1))
        } else {
            Vec::new()
        };
        let mcid = self
            .property_list(resources, operands.get(1))
            .and_then(|list| self.document.get_key(&list, "MCID").as_integer());
        self.open_marking(mcid);
        marked.push(Marked {
            hides,
            starts_at: self.text.len(),
            mcid,
            // §14.9.4's replacement text, which belongs to *extraction* and not to
            // drawing: the marks are unchanged and what a reader copies is not.
            actual_text,
            described,
            artifact,
            reversed,
            associated,
        });
        if hides {
            self.hidden = self.hidden.saturating_add(1);
        }
        if reversed {
            self.reversed_chars = self.reversed_chars.saturating_add(1);
        }
    }

    /// `EMC`: closes the innermost marked-content section, and records what it enclosed over the
    /// range of the readback it covered.
    fn end_marked_content(
        &mut self,
        marked: &mut Vec<Marked>,
        replaced_ends_at: &mut Option<usize>,
    ) {
        if let Some(section) = marked.pop() {
            if section.hides {
                self.hidden = self.hidden.saturating_sub(1);
            }
            if section.reversed {
                self.reversed_chars = self.reversed_chars.saturating_sub(1);
            }
            // "The ActualText value shall be used as a replacement, not a
            // description, for the content" — so whatever the enclosed operators
            // read back is discarded and the stated text stands in its place.
            let replaced_here = section.actual_text.is_some();
            if let Some(replacement) = section.actual_text
                && section.starts_at <= self.text.len()
            {
                self.cut_replaced(section.starts_at, &replacement);
                // "If each of two (or more) consecutive structure or marked-content
                // sequences has an ActualText entry, they shall be treated as if no
                // word break is present between them." The space between them is no
                // content of either sequence — it is what the *placement* pass
                // inferred from the gap the glyphs left, which the cut above kept at
                // this section's start (ADR 1948) — so the clause is asking for it to
                // go. Only whitespace is removed, and only where the previous
                // section's replacement ended where this one's text began: a real
                // character between the two means they are not consecutive.
                if let Some(end) = *replaced_ends_at
                    && self.text.get(end..).is_some_and(|between| {
                        !between.is_empty() && between.chars().all(char::is_whitespace)
                    })
                {
                    self.truncate_readback(end);
                }
                self.push_replacement(&replacement);
                *replaced_ends_at = Some(self.text.len());
            } else if !replaced_here && self.text.len() > section.starts_at {
                // Marks were made that no `/ActualText` replaced, so whatever came
                // before is no longer adjacent to whatever comes next.
                *replaced_ends_at = None;
            }
            // §14.9.3's and §14.9.5's substitutions are recorded over the range
            // rather than applied to it: they are what the page is *spoken* as,
            // and the text a person copies is unchanged by either.
            if let Some(described) = section.described {
                self.described.push(crate::accessibility::Described {
                    range: section.starts_at..self.text.len(),
                    alt: described.alt,
                    expansion: described.expansion,
                    language: described.language,
                });
            }
            // Last, because the range is over the readback *as it now stands*:
            // an artifact whose section also states an `/ActualText` has had its
            // text replaced by the block above, and a range taken before that
            // would name the text the replacement removed.
            if let Some(artifact) = section.artifact {
                self.artifacts.push(ArtifactSpan {
                    range: section.starts_at..self.text.len(),
                    artifact,
                    // §14.8.2.2.2's first sentence: the producer said so, with one of
                    // the two methods it names.
                    found: ArtifactSource::Declared,
                });
            }
            // §14.7.5.2's identifier over the same range, for the same reason and
            // after the same replacements: this is where a structure element's
            // content actually is in the readback.
            let closed = self.close_marking(section.mcid);
            if let Some(mcid) = section.mcid
                && let Some(closed) = closed
            {
                self.marked.push(MarkedSpan {
                    mcid,
                    // The identifier is unique "within its content stream" and no
                    // further, so which stream this is is half the key.
                    stream: self.stream,
                    range: section.starts_at..self.text.len(),
                    drawn: closed.drawn,
                    enclosed_a_refusal: closed.enclosed_a_refusal,
                });
            }
            // §14.13.5's files belong to the graphics objects the section enclosed,
            // so like an artifact they are recorded over the range rather than
            // changing anything drawn.
            for attachment in section.associated {
                self.associated
                    .push((section.starts_at..self.text.len(), attachment));
            }
        }
    }
}
