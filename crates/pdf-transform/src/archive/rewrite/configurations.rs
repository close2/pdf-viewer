//! Optional-content configurations: the names a configuration must state and the order it must complete.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::{Rewrite, Rewriter, count, name_string};

impl Rewriter<'_> {
    /// The `/Name` the operator's wording gives a configuration stating none, wherever this
    /// object holds one (`doc/adr/1952`).
    ///
    /// [`Self::remove_automatic_states`]' two places: the configuration that is this object, and
    /// the `/OCProperties` object holding some directly. The third is
    /// [`Self::name_the_catalogs_configurations`]'.
    pub(super) fn name_configurations(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::ConfigurationNamed) {
            return false;
        }
        let names = &self.remedies.configuration_names;
        if let Some(name) = names.at.get(&id) {
            out.insert(Name::new(&b"Name"[..]), name_string(name));
            count(applied, Rewrite::ConfigurationNamed);
            return true;
        }
        if names.properties == Some(id) {
            return Self::name_direct_configurations(out, &names.direct, applied);
        }
        false
    }

    /// The names of configurations written inside an `/OCProperties` the catalog states directly.
    pub(super) fn name_the_catalogs_configurations(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let names = &self.remedies.configuration_names;
        if !self.wants(Rewrite::ConfigurationNamed) || !names.in_catalog {
            return false;
        }
        let Some(Object::Dictionary(properties)) = catalog.get("OCProperties") else {
            return false;
        };
        let mut properties = properties.clone();
        if !Self::name_direct_configurations(&mut properties, &names.direct, applied) {
            return false;
        }
        catalog.insert(
            Name::new(&b"OCProperties"[..]),
            Object::Dictionary(properties),
        );
        true
    }

    /// Each configuration written in place inside one `/OCProperties` dictionary, named.
    fn name_direct_configurations(
        properties: &mut Dictionary,
        direct: &BTreeMap<Option<usize>, String>,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if let Some(name) = direct.get(&None)
            && let Some(Object::Dictionary(default)) = properties.get("D")
        {
            let mut default = default.clone();
            default.insert(Name::new(&b"Name"[..]), name_string(name));
            properties.insert(Name::new(&b"D"[..]), Object::Dictionary(default));
            count(applied, Rewrite::ConfigurationNamed);
            changed = true;
        }
        if let Some(Object::Array(items)) = properties.get("Configs") {
            let mut rebuilt = items.clone();
            let mut any = false;
            for (index, item) in rebuilt.iter_mut().enumerate() {
                if let (Some(name), Object::Dictionary(configuration)) =
                    (direct.get(&Some(index)), &*item)
                {
                    let mut configuration = configuration.clone();
                    configuration.insert(Name::new(&b"Name"[..]), name_string(name));
                    *item = Object::Dictionary(configuration);
                    count(applied, Rewrite::ConfigurationNamed);
                    any = true;
                }
            }
            if any {
                properties.insert(Name::new(&b"Configs"[..]), Object::Array(rebuilt));
                changed = true;
            }
        }
        changed
    }

    /// An optional content configuration's completed `/Order`, or the `/OCProperties` object
    /// holding configurations written directly inside it.
    pub(super) fn complete_order(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(orders) = self
            .orders
            .filter(|_| self.wants(Rewrite::OptionalContentOrder))
        else {
            return false;
        };
        if let Some(order) = orders.at.get(&id) {
            out.insert(Name::new(&b"Order"[..]), Object::Array(order.clone()));
            count(applied, Rewrite::OptionalContentOrder);
            return true;
        }
        if let Some((at, properties)) = &orders.properties
            && *at == id
        {
            *out = properties.clone();
            count(applied, Rewrite::OptionalContentOrder);
            return true;
        }
        false
    }
}
