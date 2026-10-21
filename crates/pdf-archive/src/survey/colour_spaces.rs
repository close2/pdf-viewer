//! A colour space selected by name or by a device-colour operator, and the default space a family resolves to.

use crate::finding::Where;
use pdf_syntax::Dictionary;
use pdf_syntax::Name;
use pdf_syntax::Object;

use super::{
    Cmyk, Context, DeviceFamily, Route, Selected, SpaceKind, State, Walk, classify, icc_profile,
    is_pattern, names_a_family, reaches_process_cmyk, resolve_space,
};

impl Walk<'_> {
    /// A device colour operator — `g`, `rg`, `k` and their stroking twins — which selects its
    /// family's space and a colour in it at once (§8.6.8).
    pub(super) fn set_device_colour(
        &mut self,
        family: DeviceFamily,
        stroking: bool,
        state: &mut State,
        context: &Context<'_>,
    ) {
        self.record_colour(
            (family, Route::Direct),
            context,
            "a colour operator",
            state.blending.kind,
        );
        let cmyk = if family == DeviceFamily::Cmyk {
            Cmyk::Process
        } else {
            Cmyk::Neither
        };
        if stroking {
            state.stroke = Selected::Plain;
            state.stroke_cmyk = cmyk;
        } else {
            state.fill = Selected::Plain;
            state.fill_cmyk = cmyk;
        }
    }

    /// A `cs` or `CS` operator, which selects a colour space by name.
    ///
    /// §8.6.8 lets the operand be one of the four family names outright; anything else is a key
    /// into the `/ColorSpace` subdictionary of the resources in force, and a key that reaches
    /// nothing is a resource the stream referenced and its resources did not define.
    pub(super) fn set_space(
        &mut self,
        name: &[u8],
        stroking: bool,
        state: &mut State,
        context: &Context<'_>,
    ) {
        let object = Object::Name(Name::new(name.to_vec()));
        let resolved = if names_a_family(name) {
            object
        } else {
            match self.look_up("ColorSpace", name, context, context.what) {
                Some(entry) => resolve_space(self.document, &entry, context.resources, 0),
                None => return,
            }
        };
        let selected = if is_pattern(self.document, &resolved) {
            Selected::Pattern
        } else {
            Selected::Plain
        };
        let profile = icc_profile(self.document, &resolved, context.resources, 0);
        let cmyk = if profile
            .as_ref()
            .is_some_and(|profile| profile.family == Some(DeviceFamily::Cmyk))
        {
            Cmyk::Icc
        } else if reaches_process_cmyk(self.document, &resolved, 0) {
            Cmyk::Process
        } else {
            Cmyk::Neither
        };
        if stroking {
            state.stroke = selected;
            state.stroke_cmyk = cmyk;
        } else {
            state.fill = selected;
            state.fill_cmyk = cmyk;
        }
        let place = Where::page(context.page);
        self.record_space(
            &resolved,
            &place,
            context,
            "a colour space operator",
            state.blending.kind,
            state.blending.profile.as_ref(),
        );
    }

    /// §8.6.5.6's default colour space for one family, as the resources in force state it.
    pub(super) fn default_space(
        &self,
        resources: &Dictionary,
        family: DeviceFamily,
    ) -> Option<SpaceKind> {
        let table = self.document.get_key(resources, "ColorSpace");
        let stated = table.as_dict()?.get(family.default_key())?.clone();
        let stated = self.document.resolve(&stated);
        Some(classify(self.document, &stated, resources, 0))
    }
}
