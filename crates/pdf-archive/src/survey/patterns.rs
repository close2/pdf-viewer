//! A pattern or a shading selected as a colour, and the colour space its shading names.

use crate::finding::Where;
use pdf_model::content::ledger::Route as Via;
use pdf_syntax::Dictionary;
use pdf_syntax::Object;

use super::{Context, State, Walk};

impl Walk<'_> {
    /// A `scn` or `SCN` operand naming a pattern, which is content or a shading of its own.
    pub(super) fn pattern(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("Pattern", name, context, context.what) else {
            return;
        };
        let id = entry.as_reference();
        match self.document.resolve(&entry) {
            // A tiling pattern is a content stream, read against its own resources — which is
            // why a `/DefaultCMYK` on the page does not reach into one — and against nothing
            // where it states none: Table 74 makes the entry "( Required )", and no sentence
            // gives a pattern the fallback §7.8.3 gives a form XObject, a Type 3 font and
            // (since Errata Collection 3's NOTE 3) an annotation appearance stream. So every
            // name such a cell uses is a resource the file never defined, which is what
            // `pdf_model`'s interpreter has always reported and what this walk reads now.
            // ADR 1059.
            Object::Stream(stream) => {
                let own = self.document.get_key(&stream.dict, "Resources");
                let stated = own.as_dict().is_some();
                let resources = own.as_dict().cloned().unwrap_or_default();
                self.nested(
                    &stream,
                    id,
                    &resources,
                    context.page,
                    context.depth,
                    state,
                    "a tiling pattern",
                    stated,
                    Via::TilingPattern,
                    false,
                );
            }
            Object::Dictionary(dict) => self.shading(&dict, state, context),
            _ => {}
        }
    }

    /// A `sh` operator's operand, which names a shading in the resources.
    pub(super) fn shading_resource(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("Shading", name, context, context.what) else {
            return;
        };
        let dict = match self.document.resolve(&entry) {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => return,
        };
        self.record_shading_space(&dict, state, context);
    }

    /// A shading pattern's `/Shading`, which is where its colour space is stated.
    fn shading(&mut self, pattern: &Dictionary, state: &State, context: &Context<'_>) {
        let shading = self.document.get_key(pattern, "Shading");
        let dict = match shading {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => return,
        };
        self.record_shading_space(&dict, state, context);
    }

    /// One shading dictionary's `/ColorSpace`: §8.7.4.3, Table 77.
    fn record_shading_space(&mut self, shading: &Dictionary, state: &State, context: &Context<'_>) {
        let Some(stated) = shading.get("ColorSpace") else {
            return;
        };
        let stated = self.document.resolve(stated);
        let place = Where::page(context.page);
        self.record_space(
            &stated,
            &place,
            context,
            "a shading",
            state.blending.kind,
            state.blending.profile.as_ref(),
        );
    }
}
