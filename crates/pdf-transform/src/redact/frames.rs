//! Entering a form `XObject` and the frame a stream runs in, ISO 32000-2 §8.10 and §12.5.6.23.
//!
//! `Do` is where the walk leaves one content stream for another: it decides between an image
//! and a form, enters the form under its `/Matrix` and `/Resources` (§8.10.1), saves and
//! restores the walk's state around it ([`Walk::enter_frame`], [`Walk::leave_frame`]) and records
//! the edit the form asked for, so a form drawn more than once can be split ([`super::forms`]).

use pdf_render::Transform;
use pdf_syntax::object::{Dictionary, Name, Object};

use super::{
    AddedForms, AddedMasks, AddedStates, Edits, Entered, FormEdit, Frame, GraphicsState, MAX_DEPTH,
    Operand, Overrides, TextState, Walk, apply_edits, overlaps, stated_matrix,
};

impl Walk<'_> {
    /// `Do`: an image that meets the region is cleared (§12.5.6.23); a form or an image the
    /// removal cannot clear is refused by its own narrower reason.
    pub(super) fn do_xobject(
        &mut self,
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((name, name_start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Name(bytes) => Some((bytes.clone(), *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        let xobjects = self.document.get_key(&self.resources, "XObject");
        let Some(entry) = xobjects
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .cloned()
        else {
            return Err(format!(
                "the content draws /{} which /Resources /XObject does not define; the page is \
                 refused",
                String::from_utf8_lossy(&name)
            ));
        };
        let object = self.document.resolve(&entry);
        let Some(dict) = object.as_dict() else {
            return Err(format!(
                "the content draws /{}, which does not resolve to a stream; the page is refused",
                String::from_utf8_lossy(&name)
            ));
        };
        let subtype = self
            .document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();
        match subtype.as_slice() {
            b"Image" => {
                if !self
                    .regions
                    .iter()
                    .any(|region| overlaps(*region, self.unit_square()))
                {
                    // The image is nowhere near a redaction: it crosses the output untouched.
                    return Ok(());
                }
                let clears = self.plan_image_clear(&name, &entry, &object)?;
                self.clears.extend(clears);
                Ok(())
            }
            b"Form" => self.run_form(
                &name,
                &entry,
                &object,
                Entered::Drawn {
                    name_start,
                    do_end: keyword_start.saturating_add(2),
                },
            ),
            _ => {
                if self
                    .regions
                    .iter()
                    .any(|region| overlaps(*region, self.unit_square()))
                {
                    return Err(format!(
                        "§12.5.6.23: a /{} XObject meets the region, whose content the removal \
                         does not reach; the page is refused",
                        String::from_utf8_lossy(&subtype)
                    ));
                }
                Ok(())
            }
        }
    }

    /// `Do` on a form `XObject` (§8.10): the walk **enters** the form's own content stream.
    ///
    /// A form is "a self-contained description of any sequence of graphics objects" (§8.10.1), so
    /// the marks it draws under the region are described in *its* stream and removing them is an
    /// edit of that stream. Entering it is not optional even for a form the region misses: the
    /// interpreter runs a form's content inline, so every code it shows is in the placed
    /// quadrilaterals this walk is held to, and a walk that skipped the form would disagree with
    /// the interpreter's count and refuse every page whose text is inside one.
    ///
    /// §8.10.1's step b) concatenates the form's `/Matrix` with the CTM before its content runs,
    /// and Table 93's `/Resources` is what its names resolve in — the page's dictionary where it
    /// states none, which is the direction §7.8.3's NOTE 3 gives and the reading ADR 0255 fixed.
    /// The graphics state is otherwise inherited, so the text state a `Tf` outside the form set
    /// is still in force inside it.
    ///
    /// Refused by name: a form whose content does not decode, and a form that draws itself
    /// (§8.10.1 states no recursion, and a walk that followed one would not end).
    pub(super) fn run_form(
        &mut self,
        name: &[u8],
        entry: &Object,
        object: &Object,
        entered: Entered,
    ) -> Result<(), String> {
        let shown = String::from_utf8_lossy(name);
        let Some(stream) = object.as_stream() else {
            return Err(format!(
                "the content draws the form /{shown}, which is not a stream; the page is refused"
            ));
        };
        let form_id = entry.as_reference();
        if let Some(id) = form_id
            && self.forms_open.contains(&id)
        {
            return Err(format!(
                "§8.10.1: the form /{shown} draws itself; the page is refused rather than walked \
                 for ever"
            ));
        }
        if self.forms_open.len() >= MAX_DEPTH {
            return Err(format!(
                "§8.10.1: forms are nested deeper than this removal walks at /{shown}; the page \
                 is refused"
            ));
        }
        let Some(data) = self.document.decoded_stream_data(stream) else {
            return Err(format!(
                "§8.10.1: the form /{shown} did not decode, so what it draws under the region \
                 cannot be read; the page is refused"
            ));
        };

        let matrix = stated_matrix(self.document, &stream.dict);
        let resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| self.page.resources.clone());

        let group = self
            .document
            .get_key(&stream.dict, "Group")
            .as_dict()
            .is_some_and(|group| {
                self.document
                    .get_key(group, "S")
                    .as_name()
                    .is_some_and(|name| name.as_bytes() == b"Transparency")
            });
        let saved = self.enter_frame(resources, matrix.then(self.ctm), entered, group);
        let shared = !form_id.is_some_and(|id| self.owns(id));
        if let Some(id) = form_id {
            self.forms_open.push(id);
        }
        if shared {
            self.inside_shared = self.inside_shared.saturating_add(1);
        }
        let walked = self.run_stream(&data);
        if shared {
            self.inside_shared = self.inside_shared.saturating_sub(1);
        }
        if form_id.is_some() {
            self.forms_open.pop();
        }
        let (edits, states, (named, overrides, masks), resources) =
            self.leave_frame(saved, entered);
        walked?;

        let edited = (!edits.is_empty()).then(|| apply_edits(&data, edits));
        if let Some(id) = form_id {
            self.form_outcomes.push((id, edited.clone()));
        }
        let Some(content) = edited else {
            // Nothing of this form falls under the region: its stream crosses the output byte
            // for byte, and so does the `Do` that draws it.
            return match form_id {
                Some(id) if self.split.contains_key(&id) => self.place_split(id, None, entered),
                _ => Ok(()),
            };
        };
        let Some(id) = form_id else {
            return Err(format!(
                "§12.5.6.23: the form /{shown} draws marks under the region but is a direct \
                 object this removal cannot replace; the page is refused"
            ));
        };
        // A mask's group is reached through an `/ExtGState` entry, a graphics state dictionary
        // and a soft-mask dictionary, any of which another page may share; a copy for this page
        // is correct whichever of them is shared, because the original is carried into the
        // output only where something else still reaches it.
        let private = matches!(entered, Entered::MaskGroup { .. }) || !self.owns(id);
        let edit = FormEdit {
            id,
            content,
            private,
            resources,
            states,
            forms: named,
            overrides,
            masks,
            placement_copy: false,
        };
        if self.split.contains_key(&id) {
            return self.place_split(id, Some(edit), entered);
        }
        self.record_form_edit(edit)
    }

    /// Sets the page's per-stream state aside and starts a form's, returning what to put back.
    ///
    /// §8.10.1: a form inherits the graphics state, and its own `q`/`Q` nesting is its own — so
    /// the state carries in and the stack is set aside. Two things differ from inheritance: a
    /// transparency group starts from Table 51's initial alpha constants, which the table says a
    /// reader resets at the beginning of such a group's execution, and a soft mask's group is evaluated from the initial graphics state,
    /// text state included, under its own coordinate system (§11.6.5.1), which is what the
    /// interpreter runs it under.
    fn enter_frame(
        &mut self,
        resources: Dictionary,
        ctm: Transform,
        entered: Entered,
        group: bool,
    ) -> Frame {
        let saved = Frame {
            edits: std::mem::take(&mut self.edits),
            resources: std::mem::replace(&mut self.resources, resources),
            ctm: std::mem::replace(&mut self.ctm, ctm),
            ctm_stack: std::mem::take(&mut self.ctm_stack),
            path: self.path.take(),
            graphics: self.graphics.clone(),
            graphics_stack: std::mem::take(&mut self.graphics_stack),
            text: self.text_state(),
            added_states: std::mem::take(&mut self.added_states),
            added_forms: std::mem::take(&mut self.added_forms),
            added_overrides: std::mem::take(&mut self.added_overrides),
            added_masks: std::mem::take(&mut self.added_masks),
            stream_base: std::mem::replace(&mut self.stream_base, ctm),
        };
        if group {
            self.graphics.stroking_alpha = 1.0;
            self.graphics.fill_alpha = 1.0;
        }
        if matches!(entered, Entered::MaskGroup { .. }) {
            self.graphics = GraphicsState::default();
            self.set_text_state(TextState::default());
        }
        saved
    }

    /// Puts the enclosing stream's state back, returning the form's edits, the graphics states
    /// its edits name and the resources its names were resolved in.
    fn leave_frame(
        &mut self,
        saved: Frame,
        entered: Entered,
    ) -> (
        Edits,
        AddedStates,
        (AddedForms, Overrides, AddedMasks),
        Dictionary,
    ) {
        let edits = std::mem::replace(&mut self.edits, saved.edits);
        let states = std::mem::replace(&mut self.added_states, saved.added_states);
        let named = std::mem::replace(&mut self.added_forms, saved.added_forms);
        let overrides = std::mem::replace(&mut self.added_overrides, saved.added_overrides);
        let masks = std::mem::replace(&mut self.added_masks, saved.added_masks);
        self.stream_base = saved.stream_base;
        let resources = std::mem::replace(&mut self.resources, saved.resources);
        self.ctm = saved.ctm;
        self.ctm_stack = saved.ctm_stack;
        self.path = saved.path;
        self.graphics = saved.graphics;
        self.graphics_stack = saved.graphics_stack;
        if matches!(entered, Entered::MaskGroup { .. }) {
            self.set_text_state(saved.text);
        }
        (edits, states, (named, overrides, masks), resources)
    }

    /// Records one form's edited content in the form's own place.
    ///
    /// A form drawn twice on the page is one stream, so this place can hold it only one way: two
    /// placements asking for the same edit are one edit. Two asking for different ones are what
    /// the first walk of a page notes and the second splits ([`super::forms`], ADR 1363), so on the
    /// first walk the disagreement is left for [`super::forms::split`] to find.
    pub(super) fn record_form_edit(&mut self, edit: FormEdit) -> Result<(), String> {
        if let Some(earlier) = self
            .form_edits
            .iter()
            .find(|earlier| earlier.id == edit.id && !earlier.placement_copy)
        {
            if earlier.content == edit.content {
                return Ok(());
            }
            if self.split.is_empty() {
                return Ok(());
            }
            return Err(format!(
                "§8.10.1: the second walk of the page asked form object {} for an edit the first \
                 did not; the page is refused rather than written from two readings",
                edit.id.number
            ));
        }
        self.form_edits.push(edit);
        Ok(())
    }

    /// The text state the walk tracks, as one value, for a stream that starts from its own.
    fn text_state(&self) -> TextState {
        TextState {
            linear: self.text_linear,
            size: self.font_size,
            horizontal: self.horizontal,
            char_spacing: self.char_spacing,
            word_spacing: self.word_spacing,
            codes: self.codes.clone(),
        }
    }

    /// Puts a text state back.
    fn set_text_state(&mut self, state: TextState) {
        self.text_linear = state.linear;
        self.font_size = state.size;
        self.horizontal = state.horizontal;
        self.char_spacing = state.char_spacing;
        self.word_spacing = state.word_spacing;
        self.codes = state.codes;
    }
}
