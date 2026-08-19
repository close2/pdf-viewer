//! `Do`: form `XObject`s (§8.10) and the entries their dictionaries share.
//!
//! An image `XObject` is dispatched from here and drawn by [`super::image`]; a form with a
//! `/Group` is handed to [`super::transparency`].

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Object};

use super::report::Unsupported;
use super::run::{name_at, narrow};
use super::{GraphicsState, Interpreter, MAX_FORM_DEPTH};

impl Interpreter<'_> {
    /// Draws an `XObject`: a form is interpreted inline, an image is reported.
    ///
    /// §8.8.1's Table 86 states two requirements on what `Do`'s operand finds, and a file that
    /// breaks either draws nothing where the producer asked for a mark:
    ///
    /// > Paint the specified XObject . The operand name shall appear as a key in the XObject
    /// > subdictionary of the current resource dictionary (see 7.8.3, "Resource
    /// > dictionaries"). The associated value shall be a stream whose Type entry, if present,
    /// > is XObject .
    ///
    /// **Both were silent until the four-hundred-and-nineteenth session**, which is trap 5's
    /// shape: a missing *font* has said "no /Font resource named /F1" since the interpreter
    /// had fonts, and `sh` has said "/Sh0 is not in /Shading", so a page that names an
    /// undefined `XObject` was the one resource category out of the three whose absence
    /// looked exactly like a page the producer meant to leave sparse. §7.8.3 makes the file
    /// wrong rather than this reader — "[a] content stream's named resources shall be defined
    /// by a resource dictionary, which shall enumerate the named resources needed by the
    /// operators in the content stream" — and this tree's rule for a malformed file is to draw
    /// what can be drawn and say what could not.
    pub(super) fn draw_xobject(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &GraphicsState,
        form_depth: usize,
    ) {
        let Some(name) = name_at(operands, 0) else {
            // Table 86's operand column is `name`, so a `Do` with anything else — or with
            // nothing — is not a `Do` at all. Reported by operator rather than by resource,
            // because there is no name to say was undefined.
            self.note(Unsupported::Operator {
                operator: "Do with no name operand".to_owned(),
            });
            return;
        };
        let Some(object) = self.resource(resources, "XObject", &name) else {
            self.note_missing_resource("XObject", &name, "is not in /XObject");
            return;
        };
        let Some(stream) = object.as_stream().cloned() else {
            self.note_missing_resource("XObject", &name, "is not a stream");
            return;
        };

        // §8.11.3.3: a form or image XObject may carry an `/OC` entry naming a group or a
        // membership dictionary, and its visibility is that of the group "along with the
        // current visibility state in the context in which the XObject is invoked" — which
        // is what `is_hidden` already carries. §8.11.3.1 permits skipping such an object
        // entirely, because a form's state changes do not outlive it, and skipping is what
        // keeps an undrawable image inside a hidden layer from being reported as a gap.
        // Read unresolved: a group is identified by *which object* it is (§8.11.2.2).
        let group = stream.dict.get("OC").cloned();
        if let Some(oc) = &group
            && !self.shows_optional_content(oc)
        {
            // §8.9.5.4 step a), as Errata Collection 3 amends it: "[i]f the base image contains
            // an OC entry that specifies that the content is not visible, then nothing shall be
            // shown." Terminal — the `/Alternates` are **not** examined, because steps c) and d)
            // begin at "Otherwise" and are reached only by an image that states no `/OC` at all.
            // Step b) is the fall-through below, since drawing the base image is what it asks
            // for. `Interpreter::alternate_image` carries the whole amended algorithm.
            return;
        }
        if self.is_hidden() {
            return;
        }

        // What a *report* and an image's diagnostic label want is text; §7.3.5 binds the lookups
        // above, which take the name's own bytes.
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();

        if subtype == b"Image" {
            // §8.9.5.4 steps d) and e): a base image stating no `/OC` may be replaced by the
            // first of its `/Alternates` whose own `/OC` says it is visible, and where none is
            // identified "the base image shall be rendered".
            let alternate = match group {
                Some(_) => None,
                None => self.alternate_image(&stream.dict, &label),
            };
            let drawn = alternate.as_ref().unwrap_or(&stream);
            // An `XObject` is an object the resource dictionary hands out, so the same `Do`
            // twice is the same allocation twice and its address is the name of the image.
            self.draw_image(
                crate::image::NamedStream::allocation(drawn),
                &label,
                resources,
                state,
            );
            return;
        }
        if subtype != b"Form" {
            self.note(Unsupported::Operator {
                operator: format!("Do on /{label}"),
            });
            return;
        }

        if form_depth >= MAX_FORM_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH",
            });
            return;
        }

        // §8.10.1 makes a form "a self-contained description of any sequence of graphics
        // objects", which is §7.8.2's content stream under another name — so a form that
        // decoded only part-way is drawn to where its damage is and reported for the rest.
        // See [`Interpreter::content_stream`].
        let Some(data) = self.content_stream(&stream, &format!("a form XObject /{label} (§8.10)"))
        else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable form /{label}"),
            });
            return;
        };

        // A form carries its own matrix and its own resources, falling back to the page's.
        let mut inner = state.clone();
        if let Some(matrix) = self.matrix(&stream.dict) {
            inner.transform = matrix.then(inner.transform);
        }

        // §8.10.1 lists what `Do` performs on a form XObject, and step c) is "Clips
        // according to the form dictionary's BBox entry"; Table 93 says of `/BBox` that
        //
        // > These boundaries shall be used to clip the form XObject and to determine its
        // > size for caching.
        //
        // Required of every form, not only of an annotation's appearance — which is the one
        // place this tree had it. §11.6.6 needs it too: a group's shape is the union of its
        // elements "clipped by the group XObject's bounding box".
        //
        // A form with no `/BBox` is malformed, since Table 93 makes it required. It is drawn
        // unclipped rather than refused: there is no box to honour, and the alternative
        // reading — clip to nothing — would delete content the producer plainly meant to
        // draw.
        if let Some(bbox) = self.rectangle(&stream.dict, "BBox") {
            let Some(clip) = self.rect_clip(bbox, inner.transform, inner.clip) else {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return;
            };
            inner.clip = Some(clip);
        }

        // A form that omits `/Resources` is looked up in its parent's, which §7.8.3's NOTE 3
        // reports of earlier versions of PDF and Table 93 now makes "Sometimes required" rather
        // than "Optional but strongly recommended" (Errata Collection 3, Issues #128 and #292).
        // **The fallback is on the entry's absence and not on a name's**: a form that states a
        // `/Resources` has stated which names it uses, so a name that dictionary omits is
        // reported by `draw_xobject` above rather than looked up a second time here. Both
        // readings are choices about a malformed file — the standard defines neither — and this
        // one is the same choice `font` makes for `Tf`, which matters because the alternative
        // is what session 127 had to undo: a page's `/Fm0` and a form's `/Fm0` are two objects
        // as often as they are one, and reaching past the dictionary that names them is how a
        // reader draws the wrong one and says nothing.
        let form_resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| resources.clone());

        // §8.7.2: a pattern's matrix maps pattern space to "the default coordinate system of
        // the pattern's parent content stream", and the clause says what that means here:
        //
        // > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects"
        // > ), the pattern matrix maps pattern space to the form's default user space (that
        // > is, the form coordinate space at the time the form is painted with the Do
        // > operator).
        //
        // Which is `inner.transform`: §8.10.1's step b) concatenates the form's `/Matrix`
        // with the CTM before its content stream runs, so the form's default user space is
        // the space that content starts in. Restored afterwards, because the *page's*
        // default space is what a pattern used on the page maps to and the two are different
        // spaces with the same name.
        let outer_base = std::mem::replace(&mut self.base, inner.transform);
        let Some(group) = self.transparency_group(&stream.dict) else {
            self.run(&data, &form_resources, &inner, form_depth.saturating_add(1));
            self.base = outer_base;
            return;
        };
        self.run_transparency_group(&group, &data, &form_resources, &inner, state, form_depth);
        self.base = outer_base;
    }

    /// Reads a form dictionary's `/Matrix` (§8.10.2 Table 93).
    ///
    /// > An array of six numbers specifying the form matrix , which maps form space into
    /// > user space (see 8.3.4, "Transformation matrices").
    ///
    /// Shared by the two places that need it, because §11.6.5.1 defines a soft mask's
    /// coordinate system as this matrix concatenated with the transform in force at the
    /// `gs` — the same reading of the same entry that `Do` makes, and one worth making
    /// once.
    pub(super) fn matrix(&mut self, dict: &Dictionary) -> Option<Transform> {
        let entry = self.document.get_key(dict, "Matrix");
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 6).then(|| {
            Transform::new(
                values[0], values[1], values[2], values[3], values[4], values[5],
            )
        })
    }

    /// Reads a rectangle entry as four numbers, in the order the file wrote them.
    pub(super) fn rectangle(&mut self, dict: &Dictionary, key: &str) -> Option<[f32; 4]> {
        let entry = self.document.get_key(dict, key);
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 4).then(|| [values[0], values[1], values[2], values[3]])
    }
}
