//! An `ExtGState` entry as the walk reads it: overprint parameters and whether a state involves transparency.

use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use pdf_syntax::ObjectId;

use pdf_model::content::ledger::Route as Via;

use super::{Context, State, Walk, below_one, blends};

impl Walk<'_> {
    /// A `gs` operator: Annex Q.2's four conditions on the graphics state it sets.
    pub(super) fn graphics_state(&mut self, name: &[u8], state: &mut State, context: &Context<'_>) {
        let Some(entry) = self.look_up("ExtGState", name, context, context.what) else {
            return;
        };
        let Object::Dictionary(dict) = self.document.resolve(&entry) else {
            return;
        };
        if self.is_transparent_state(&dict) {
            self.survey.transparent.insert(context.page);
        }
        self.overprint(&dict, entry.as_reference(), state);
        self.soft_mask(&dict, context);
    }

    /// The transparency group of the soft mask a `gs` sets, walked as the content it is.
    ///
    /// ISO 32000-2 §11.6.5.1 makes the mask's `/G` a transparency group `XObject`, so its content
    /// selects resources and paints colours exactly as a form's does, and a colour it paints is a
    /// use of that colour space under ISO 19005-2 section 6.2.4.3 and ISO 19005-4 section 6.2.4.3,
    /// neither of which exempts a mask's group (`doc/adr/2263`). It is walked where the `gs` sets
    /// it, which is where §11.6.5.1 fixes the mask and where the interpreter evaluates it, and it
    /// starts from §8.4.1's initial state rather than the invoking stream's: the group is not
    /// painted into the stream that set it, so nothing of that stream's colour or blending space
    /// reaches inside (§11.5.3 turns its result into one luminosity or alpha per point).
    ///
    /// The soft-mask *image* an image's `/SMask` names stays outside the walk, for the reason the
    /// module comment gives.
    fn soft_mask(&mut self, dict: &Dictionary, context: &Context<'_>) {
        let mask = self.document.get_key(dict, "SMask");
        let Some(mask) = mask.as_dict() else {
            return;
        };
        let group = mask.get("G").and_then(Object::as_reference);
        let Object::Stream(stream) = self.document.get_key(mask, "G") else {
            return;
        };
        self.form(
            &stream,
            group,
            context.page,
            context.depth,
            &State::default(),
            Via::SoftMask,
        );
    }

    /// ISO 32000-2 §8.4.5, Table 58's `OP`, `op` and `OPM`, as a `gs` operator sets them.
    ///
    /// `OP` sets both parameters or only the stroking one, and which of the two it does is
    /// decided by the same dictionary rather than by what any earlier one said:
    ///
    /// > Specifying an OP entry shall set both parameters unless there is also an op entry in
    /// > the same graphics state parameter dictionary, in which case the OP entry shall set only
    /// > the overprint parameter for stroking.
    ///
    /// `id` is the dictionary's object where it is one, kept beside the mode it sets so that a
    /// caller changing the mode knows which object to change.
    fn overprint(&self, dict: &Dictionary, id: Option<ObjectId>, state: &mut State) {
        let stated = |key: &str| dict.get(key).is_some();
        let on = |object: &Object| *object == Object::Boolean(true);
        if stated("OP") {
            let stroking = on(&self.document.get_key(dict, "OP"));
            state.overprint.stroke = stroking;
            if !stated("op") {
                state.overprint.fill = stroking;
            }
        }
        if stated("op") {
            state.overprint.fill = on(&self.document.get_key(dict, "op"));
        }
        if let Some(mode) = self.document.get_key(dict, "OPM").as_integer() {
            state.overprint.mode = mode;
            state.overprint.mode_set_by = id;
        }
    }

    /// ISO 32000-2 §Q.2's four tests on one graphics state parameter dictionary.
    ///
    /// > - SMask key is present and its value is of type dictionary;
    /// > - ca key is present and its value is less than one (1);
    /// > - CA key is present and its value is less than one (1);
    /// > - BM key is present and its value is not Normal.
    fn is_transparent_state(&self, dict: &Dictionary) -> bool {
        if dict.get("SMask").is_some() && self.document.get_key(dict, "SMask").as_dict().is_some() {
            return true;
        }
        if below_one(&self.document.get_key(dict, "ca"))
            || below_one(&self.document.get_key(dict, "CA"))
        {
            return true;
        }
        blends(self.document, dict)
    }
}
