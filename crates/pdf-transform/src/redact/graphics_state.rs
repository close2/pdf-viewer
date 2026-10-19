//! The graphics state parameter dictionary and the soft masks it establishes, ISO 32000-2
//! §8.4.5, §11.6.5.2 and §12.5.6.23.
//!
//! A `gs` operator names a dictionary whose entries change the stroke's alpha and overprint
//! and may install a soft mask whose group is content that marks the page. This module reads
//! the dictionary, enters a mask's group as a stream of its own, and says whether an object may
//! be replaced in place or must be copied.

use pdf_syntax::object::{Dictionary, Name, Object};

use super::{Entered, Operand, Walk, style_code};

impl Walk<'_> {
    /// A soft mask a `gs` establishes (§11.6.4.3): its transparency group's content is walked
    /// and cut like any other content stream.
    ///
    /// §12.5.6.23 asks for "all traces of the specified content" to go, and a mask's group is
    /// content that marks the page — not in colour, but as the shape or opacity of what is
    /// painted under it — so its marks within the region are traces of what the region showed.
    /// A glyph drawn into a `/Luminosity` group is legible through every mark the mask applies
    /// to, and it would survive in the file with every one of those marks cut away. So the group
    /// is entered, in §11.6.5.1's coordinate system — "concatenating the transformation matrix
    /// specified by the Matrix entry in the transparency group's form dictionary … with the
    /// current transformation matrix at the moment the soft mask is established in the graphics
    /// state with the gs operator" — at the `gs`, which is where the interpreter evaluates it
    /// and so where its codes fall in the count this walk is held to. ADR 1352.
    fn enter_soft_mask(&mut self, state: &Dictionary, at: (usize, usize)) -> Result<(), String> {
        let mask = self.document.get_key(state, "SMask");
        let Some(mask) = mask.as_dict() else {
            // `/None`, or no entry: §11.6.4.3's absence of a mask, which removes nothing.
            return Ok(());
        };
        let Some(entry) = mask.get("G").cloned() else {
            return Err(
                "§11.6.5.1: a soft mask with no /G group is in force; the page is refused rather \
                 than walked past a mask whose content cannot be read"
                    .to_owned(),
            );
        };
        let object = self.document.resolve(&entry);
        self.run_form(
            b"SMask /G",
            &entry,
            &object,
            Entered::MaskGroup {
                name_start: at.0,
                gs_end: at.1,
            },
        )
    }

    /// Whether the page's `/XObject` resource subdictionary is not itself a shared object.
    pub(super) fn xobject_private(&self) -> bool {
        let resources = self.document.get_key(&self.page.dict, "Resources");
        match resources.as_dict().and_then(|dict| dict.get("XObject")) {
            Some(Object::Reference(id)) => self.counts.get(&id.number).copied() == Some(1),
            _ => true,
        }
    }

    /// `gs`: the named graphics state's line parameters and alphas, and its soft mask entered.
    pub(super) fn ext_gstate(
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
        let states = self.document.get_key(&self.resources, "ExtGState");
        let entry = states
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .cloned();
        let state = entry.as_ref().map(|entry| self.document.resolve(entry));
        let Some(dict) = state.as_ref().and_then(Object::as_dict) else {
            return Ok(());
        };
        self.current_state = entry;
        self.enter_soft_mask(dict, (name_start, keyword_start.saturating_add(2)))?;
        // Table 58 states these in §8.4.3's own terms — `/LW` "[t]he line width", `/LC` "[t]he
        // line cap style", `/LJ` "[t]he line join style", `/ML` "[t]he miter limit", `/D` "[t]he
        // line dash pattern" — so they set exactly what the operators beside them set and are
        // read into the same state. `/CA` and `/ca` are §11.6.4.4's two alpha constants, which
        // decide whether a stroke's outline may be painted as a fill at all.
        let number = |key: &str| self.document.get_key(dict, key).as_number();
        if let Some(width) = number("LW") {
            self.graphics.width = width;
        }
        if let Some(cap) = number("LC") {
            self.graphics.cap = style_code(cap);
        }
        if let Some(join) = number("LJ") {
            self.graphics.join = style_code(join);
        }
        if let Some(limit) = number("ML") {
            self.graphics.miter_limit = limit;
        }
        if let Some(alpha) = number("CA") {
            self.graphics.stroking_alpha = alpha;
        }
        if let Some(alpha) = number("ca") {
            self.graphics.fill_alpha = alpha;
        }
        // Table 58: "Specifying an OP entry shall set both parameters unless there is also an op
        // entry in the same graphics state parameter dictionary".
        let flag = |key: &str| match self.document.get_key(dict, key) {
            Object::Boolean(value) => Some(value),
            _ => None,
        };
        let (stroking, other) = (flag("OP"), flag("op"));
        if let Some(value) = stroking {
            self.graphics.stroke_overprint = value;
        }
        if let Some(value) = other.or(stroking) {
            self.graphics.fill_overprint = value;
        }
        let dash = self.document.get_key(dict, "D");
        if let Some([array, phase]) = dash.as_array() {
            let array = self.document.resolve(array);
            let dashes: Vec<f64> = array
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| self.document.resolve(item).as_number())
                        .collect()
                })
                .unwrap_or_default();
            if dashes.iter().any(|dash| *dash < 0.0) || dashes.iter().all(|dash| *dash == 0.0) {
                self.graphics.dashes = Vec::new();
                self.graphics.dash_phase = 0.0;
            } else {
                self.graphics.dashes = dashes;
                self.graphics.dash_phase =
                    self.document.resolve(phase).as_number().unwrap_or_default();
            }
        }
        Ok(())
    }
}
