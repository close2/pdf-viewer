//! Appearance states: the extra and the automatic states an annotation's `/AP` entries carry, and the
//! dictionary that is reduced to the one state a requirement admits.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::sites::PUSH_BUTTON_STATE;
use super::{Rewrite, Rewriter, count};

impl Rewriter<'_> {
    /// The one appearance stream an annotation's `/AS` selects, written as its `/N` — or a push
    /// button's single stream written as a subdictionary holding it alone, with the `/AS` that
    /// selects it (`doc/adr/2319`).
    ///
    /// The `/AP` dictionary is written **direct** whatever the source stated it as, for the
    /// reason [`Rewrite::OutputIntent`]'s array is: the value is the annotation's own and this
    /// walk rewrites objects, so an `/AP` some other object held is copied here and left behind
    /// where nothing else refers to it. Every other key of it crosses unchanged — an `/R` or a
    /// `/D` beside the normal appearance is a different requirement's business.
    pub(super) fn collapse_appearance_states(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(states) = self
            .appearance_states
            .filter(|_| self.wants(Rewrite::NormalAppearanceFromState))
        else {
            return false;
        };
        let normal = if let Some(stream) = states.at.get(&id) {
            Object::Reference(*stream)
        } else if let Some(stream) = states.wrapped.get(&id) {
            let state = Name::new(PUSH_BUTTON_STATE);
            let mut subdictionary = Dictionary::new();
            subdictionary.insert(state.clone(), Object::Reference(*stream));
            out.insert(Name::new(&b"AS"[..]), Object::Name(state));
            Object::Dictionary(subdictionary)
        } else {
            return false;
        };
        let Some(mut appearance) = self.document.get_key(out, "AP").as_dict().cloned() else {
            return false;
        };
        appearance.insert(Name::new(&b"N"[..]), normal);
        out.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearance));
        count(applied, Rewrite::NormalAppearanceFromState);
        true
    }

    /// One annotation's appearance dictionary, reduced to the normal appearance alone.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 admit `/N` and no other key, and
    /// §12.5.5's Table 170 makes the normal appearance the default of both `/R` and `/D` — so
    /// what a reader draws in those states after this is what it already draws for an annotation
    /// whose producer stated neither. The artwork is what goes, which is why the decision carries
    /// a loss (`doc/adr/1234`).
    ///
    /// The `/AP` is written **direct** whatever the source stated it as, on
    /// [`Self::collapse_appearance_states`]' reading and for its reason.
    pub(super) fn reduce_appearance_dictionary(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let reduced = self
            .extra_appearance_states
            .filter(|_| self.wants(Rewrite::ExtraAppearanceStatesRemoved));
        if reduced.is_none_or(|states| !states.at.contains(&id)) {
            return false;
        }
        let Some(appearance) = self.document.get_key(out, "AP").as_dict().cloned() else {
            return false;
        };
        let Some(normal) = appearance.get("N").cloned() else {
            return false;
        };
        let mut only_normal = Dictionary::new();
        only_normal.insert(Name::new(&b"N"[..]), normal);
        out.insert(Name::new(&b"AP"[..]), Object::Dictionary(only_normal));
        count(applied, Rewrite::ExtraAppearanceStatesRemoved);
        true
    }

    /// The `/AS` entry ISO 19005-2 section 6.9 forbids, gone from wherever this object holds one.
    ///
    /// Three shapes, because §8.11.4.3's configuration dictionaries are written at three places
    /// and `super::super::sites` found which: the configuration that is this object, and the
    /// `/OCProperties` object holding one directly. The third — an `/OCProperties` the catalog
    /// states directly — is [`Self::rewrite_catalog`]'s, since the catalog is what holds it.
    pub(super) fn remove_automatic_states(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(states) = self
            .automatic_states
            .filter(|_| self.wants(Rewrite::AutomaticStatesRemoved))
        else {
            return false;
        };
        if states.at.contains(&id) && out.remove("AS").is_some() {
            count(applied, Rewrite::AutomaticStatesRemoved);
            return true;
        }
        if states.properties == Some(id) {
            return Self::strip_automatic_states(out, applied);
        }
        false
    }

    /// The `/AS` entries of an `/OCProperties` dictionary the catalog states directly.
    ///
    /// The third of [`Self::remove_automatic_states`]' three places, here rather than there
    /// because the catalog is the object that holds it. Run after the `/Order` completion, which
    /// may have just written this dictionary: the removal edits what the catalog now states
    /// rather than what the source did.
    pub(super) fn remove_the_catalogs_automatic_states(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::AutomaticStatesRemoved)
            || !self
                .automatic_states
                .is_some_and(|states| states.in_catalog)
        {
            return false;
        }
        let Some(Object::Dictionary(properties)) = catalog.get("OCProperties") else {
            return false;
        };
        let mut properties = properties.clone();
        if !Self::strip_automatic_states(&mut properties, applied) {
            return false;
        }
        catalog.insert(
            Name::new(&b"OCProperties"[..]),
            Object::Dictionary(properties),
        );
        true
    }

    /// Every `/AS` written directly inside one `/OCProperties` dictionary, gone.
    ///
    /// §8.11.4.1's Table 98 gives the dictionary a `/D` holding one configuration and a
    /// `/Configs` holding an array of them, and a configuration reached by reference is edited in
    /// its own object instead — so what this touches is the ones written in place.
    fn strip_automatic_states(
        properties: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if let Some(Object::Dictionary(default)) = properties.get("D") {
            let mut default = default.clone();
            if default.remove("AS").is_some() {
                properties.insert(Name::new(&b"D"[..]), Object::Dictionary(default));
                count(applied, Rewrite::AutomaticStatesRemoved);
                changed = true;
            }
        }
        if let Some(Object::Array(items)) = properties.get("Configs") {
            let mut rebuilt = Vec::with_capacity(items.len());
            let mut any = false;
            for item in items {
                match item {
                    Object::Dictionary(configuration) => {
                        let mut configuration = configuration.clone();
                        if configuration.remove("AS").is_some() {
                            count(applied, Rewrite::AutomaticStatesRemoved);
                            any = true;
                        }
                        rebuilt.push(Object::Dictionary(configuration));
                    }
                    other => rebuilt.push(other.clone()),
                }
            }
            if any {
                properties.insert(Name::new(&b"Configs"[..]), Object::Array(rebuilt));
                changed = true;
            }
        }
        changed
    }
}
