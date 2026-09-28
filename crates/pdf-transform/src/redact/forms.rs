//! A form drawn more than once that a region meets differently, ISO 32000-2 §8.10.1 and
//! §12.5.6.23 (ADR 1363).
//!
//! A form `XObject` is one stream, and each `Do` draws all of it under its own transform. Where a
//! region meets two placements differently, the edit each asks of the stream is different, and no
//! single stream both removes what the region covers at one placement and keeps what it does not
//! cover at the other: written one way, it either leaves "traces of the specified content" at the
//! first placement or removes content the annotation did not identify at the second.
//!
//! So each placement is given the stream it asks for. The first walk of a page records, for every
//! placement of every form that is an object, the edited stream it asked for or that it asked for
//! nothing; a form whose placements disagree is **split** ([`split`]), and a second walk writes
//! each placement's own. A placement the region missed keeps drawing the producer's form under the
//! producer's name. An edited placement draws a **copy** — a form with the producer's dictionary
//! and the placement's own edited stream — named by a fresh resource entry its enclosing stream's
//! resources gain, and its `Do`'s operand is rewritten to that name; two placements asking for the
//! same edit share one copy. Where no placement was missed, the first edit takes the form's own
//! place, replaced or copied exactly as a form drawn once is, so the producer's stream is not left
//! in the file behind the copies.
//!
//! Inlining the edited content at each `Do` would state the same marks, but a form is also its
//! `/BBox` clip, its `/Group`, and its own resources' names, and restating those inline is a
//! rewrite of structure the producer chose; a copy keeps every entry the producer wrote and changes
//! only the stream, which is the whole of what differs. The number of copies is the number of
//! distinct edits the region asks of the form on one page, bounded by [`MAX_COPIES`]: a form a page
//! draws a hundred times is copied only as often as the region cuts it differently, and a page
//! asking for more is refused by name rather than grown without bound (principle 3).

use std::collections::HashMap;

use pdf_syntax::object::{Name, ObjectId};

use super::{Entered, FormEdit, Walk};

/// The most copies of forms one page's redaction writes before the page is refused.
///
/// Each copy is a whole content stream, so the bound is on the output's growth: a form placed on
/// a lattice that the region cuts at every placement differently would otherwise write one stream
/// per placement. Sixty-four is far above what a page asks for in practice — two placements met
/// differently is the case the construction exists for — and a page over it is refused by name.
pub(super) const MAX_COPIES: usize = 64;

/// How one placement of a split form is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slot {
    /// Under the producer's own name: the form unedited where the region missed this placement,
    /// or the form's own place taking this placement's edit.
    Original,
    /// The given copy of the form, named by a resource entry of its own.
    Copy(usize),
}

/// Which forms the first walk found drawn more than once and met differently, and how each of
/// their placements is written — an empty map where every form's placements agree.
///
/// # Errors
///
/// A refusal by name where the copies would pass [`MAX_COPIES`].
pub(super) fn split(
    outcomes: &[(ObjectId, Option<Vec<u8>>)],
) -> Result<HashMap<ObjectId, Vec<Slot>>, String> {
    let mut by_form: HashMap<ObjectId, Vec<Option<&[u8]>>> = HashMap::new();
    for (id, outcome) in outcomes {
        by_form.entry(*id).or_default().push(outcome.as_deref());
    }
    let mut out = HashMap::new();
    let mut copies = 0usize;
    for (id, placements) in by_form {
        let mut distinct: Vec<Option<&[u8]>> = Vec::new();
        for placement in &placements {
            if !distinct.contains(placement) {
                distinct.push(*placement);
            }
        }
        if distinct.len() < 2 {
            continue;
        }
        let missed = distinct.contains(&None);
        // The edits in the order first asked for; where no placement was missed, the first of
        // them takes the form's own place and the rest are copies.
        let edits: Vec<&[u8]> = distinct.iter().filter_map(|edit| *edit).collect();
        let slots: Vec<Slot> = placements
            .iter()
            .map(|placement| match placement {
                None => Slot::Original,
                Some(edit) => {
                    let index = edits.iter().position(|known| known == edit).unwrap_or(0);
                    if missed {
                        Slot::Copy(index)
                    } else if index == 0 {
                        Slot::Original
                    } else {
                        Slot::Copy(index)
                    }
                }
            })
            .collect();
        copies = copies.saturating_add(if missed {
            edits.len()
        } else {
            edits.len().saturating_sub(1)
        });
        out.insert(id, slots);
    }
    if copies > MAX_COPIES {
        return Err(format!(
            "§8.10.1: the region meets the placements of this page's forms in {copies} different \
             ways, and writing each its own copy passes this build's bound of {MAX_COPIES}; the \
             page is refused rather than grown without bound"
        ));
    }
    Ok(out)
}

impl Walk<'_> {
    /// Writes one placement of a split form the way [`split`] decided: nothing where it is the
    /// producer's form unedited, the form's own place where it takes that, and otherwise a copy
    /// under a fresh name with the `Do` rewritten to draw it.
    pub(super) fn place_split(
        &mut self,
        id: ObjectId,
        edit: Option<FormEdit>,
        entered: Entered,
    ) -> Result<(), String> {
        let reached = self.placements.entry(id).or_insert(0);
        let at = *reached;
        *reached = reached.saturating_add(1);
        let slot = self.split.get(&id).and_then(|slots| slots.get(at)).copied();
        let inconsistent = || {
            format!(
                "§8.10.1: the second walk of the page met form object {} where the first did not; \
                 the page is refused rather than written from two readings",
                id.number
            )
        };
        match (slot, edit) {
            (Some(Slot::Original), None) => Ok(()),
            (Some(Slot::Original), Some(edit)) => self.record_form_edit(edit),
            (Some(Slot::Copy(copy)), Some(mut edit)) => {
                edit.private = true;
                edit.placement_copy = true;
                let index = if let Some(index) = self.copies.get(&(id, copy)).copied() {
                    if self
                        .form_edits
                        .get(index)
                        .is_none_or(|earlier| earlier.content != edit.content)
                    {
                        return Err(inconsistent());
                    }
                    index
                } else {
                    self.form_edits.push(edit);
                    let index = self.form_edits.len().saturating_sub(1);
                    self.copies.insert((id, copy), index);
                    index
                };
                self.name_copy(entered, index);
                Ok(())
            }
            _ => Err(inconsistent()),
        }
    }

    /// Names a placement's copy in the enclosing stream: a `Do` is given a fresh `/XObject` entry
    /// naming the copy, and a soft mask's `gs` a fresh `/ExtGState` entry restating the producer's
    /// graphics state with the mask's group the copy (§11.6.5.1's `/G`); the operator's operand is
    /// rewritten to the new name either way.
    fn name_copy(&mut self, entered: Entered, index: usize) {
        let (category, prefix, start, end, operator) = match entered {
            Entered::Drawn { name_start, do_end } => {
                ("XObject", "RedactForm", name_start, do_end, "Do")
            }
            Entered::MaskGroup { name_start, gs_end } => {
                ("ExtGState", "RedactMask", name_start, gs_end, "gs")
            }
        };
        let name = self.fresh_name(category, prefix);
        let mut replacement = b"/".to_vec();
        replacement.extend_from_slice(&name);
        replacement.push(b' ');
        replacement.extend_from_slice(operator.as_bytes());
        self.edits.push((start, end, replacement));
        match entered {
            Entered::Drawn { .. } => self.added_forms.push((name, index)),
            Entered::MaskGroup { .. } => {
                let state = self
                    .current_state
                    .clone()
                    .unwrap_or(pdf_syntax::Object::Null);
                self.added_masks.push((name, state, index));
            }
        }
    }

    /// A resource name the running stream's `category` does not already hold and no earlier copy
    /// in it was given.
    fn fresh_name(&self, category: &str, prefix: &str) -> Vec<u8> {
        let taken = self.document.get_key(&self.resources, category);
        let taken = taken.as_dict();
        let mut ordinal = self
            .added_forms
            .len()
            .saturating_add(self.added_masks.len())
            .saturating_add(1);
        loop {
            let candidate = format!("{prefix}{ordinal}").into_bytes();
            let used = taken
                .is_some_and(|dict| dict.get_by_name(&Name::new(candidate.as_slice())).is_some())
                || self.added_forms.iter().any(|(name, _)| *name == candidate)
                || self
                    .added_masks
                    .iter()
                    .any(|(name, _, _)| *name == candidate);
            if !used {
                return candidate;
            }
            ordinal = ordinal.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// Placements that all ask for the same thing split nothing.
    #[test]
    fn agreeing_placements_are_not_split() {
        let edit = Some(b"a".to_vec());
        let outcomes = vec![(form(5), edit.clone()), (form(5), edit), (form(6), None)];
        assert!(split(&outcomes).expect("within the bound").is_empty());
    }

    /// A placement the region missed keeps the form's own place, so every edited placement is a
    /// copy — the producer's stream is still what the missed one draws.
    #[test]
    fn a_missed_placement_keeps_the_form_and_the_edited_ones_are_copies() {
        let outcomes = vec![
            (form(5), Some(b"a".to_vec())),
            (form(5), None),
            (form(5), Some(b"a".to_vec())),
        ];
        let slots = split(&outcomes).expect("within the bound");
        assert_eq!(
            slots.get(&form(5)).map(Vec::as_slice),
            Some([Slot::Copy(0), Slot::Original, Slot::Copy(0)].as_slice())
        );
    }

    /// Where every placement is edited, the first edit takes the form's own place.
    #[test]
    fn with_no_placement_missed_the_first_edit_takes_the_form_s_place() {
        let outcomes = vec![
            (form(5), Some(b"a".to_vec())),
            (form(5), Some(b"b".to_vec())),
        ];
        let slots = split(&outcomes).expect("within the bound");
        assert_eq!(
            slots.get(&form(5)).map(Vec::as_slice),
            Some([Slot::Original, Slot::Copy(1)].as_slice())
        );
    }

    /// More distinct edits than the bound refuse the page by name.
    #[test]
    fn more_copies_than_the_bound_are_refused() {
        let outcomes: Vec<(ObjectId, Option<Vec<u8>>)> = (0..=MAX_COPIES)
            .map(|index| (form(5), Some(index.to_le_bytes().to_vec())))
            .chain(std::iter::once((form(5), None)))
            .collect();
        let refusal = split(&outcomes).expect_err("over the bound");
        assert!(refusal.contains("bound"), "{refusal}");
    }
}
