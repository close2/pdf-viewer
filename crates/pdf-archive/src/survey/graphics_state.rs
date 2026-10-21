//! An `ExtGState` entry as the walk reads it: overprint parameters and whether a state involves transparency.

use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use pdf_syntax::ObjectId;

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
