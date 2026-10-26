//! Table 354's root entries and Table 353's mark information, reconciled across the sources
//! (the module comment of [`super`] gives the three namespaces' rules).

use std::collections::BTreeSet;

use pdf_syntax::object::{Dictionary, Object};

use super::{Carry, Host, LIST_VALUED};
use crate::{Refusal, Warning};

impl Carry {
    /// Table 354's map- and list-valued entries for one source, reconciled into the merged root.
    ///
    /// Nothing here refuses: the two maps' collisions are answered by the module comment's first
    /// two rules, and only `/IDTree`'s is a refusal — which [`Carry::check_ids`] raises once the
    /// whole hierarchy is decided, so that it names both sources rather than whichever was second.
    pub(super) fn read_root(
        &mut self,
        host: &mut dyn Host,
        at: usize,
        root: &Dictionary,
        warnings: &mut Vec<Warning>,
        source_of: &dyn Fn(usize) -> usize,
    ) {
        for entry in ["RoleMap", "ClassMap"] {
            let pairs = {
                let Some(document) = host.source(at) else {
                    return;
                };
                let Some(Object::Dictionary(dict)) = root.get(entry).map(|v| document.resolve(v))
                else {
                    continue;
                };
                dict.iter()
                    .map(|(key, value)| (key.as_bytes().to_vec(), document.resolve(value)))
                    .collect::<Vec<(Vec<u8>, Object)>>()
            };
            for (key, value) in pairs {
                if entry == "RoleMap" {
                    // §14.7.3's NOTE 1: "[t]he equivalence need not be exact; the role map merely
                    // indicates an approximate analogy between types". So a name two sources map
                    // differently keeps the first source's approximation, said out loud.
                    if let Some(held) = self.role_map.get(&key) {
                        if *held != value {
                            warnings.push(Warning {
                                source: source_of(at),
                                page: None,
                                detail: format!(
                                    "§14.7.3: /RoleMap already maps /{} and an earlier source's \
                                     mapping wins; the clause's NOTE 1 makes a role map \"an \
                                     approximate analogy between types\", so this source's \
                                     elements of that type keep the earlier equivalent",
                                    String::from_utf8_lossy(&key).escape_debug()
                                ),
                            });
                        }
                        continue;
                    }
                    let carried = host.carry_value(at, &value);
                    self.role_map.insert(key, carried);
                    continue;
                }
                // §14.7.6.2 attaches a class's attributes to the element that names it, so a
                // class two sources define differently is renamed rather than merged, and every
                // carried element's `/C` follows the rename.
                if self.class_map.get(&key).is_some_and(|held| *held == value) {
                    continue;
                }
                let carried = host.carry_value(at, &value);
                if self.class_map.contains_key(&key) {
                    let taken: BTreeSet<Vec<u8>> = self.class_map.keys().cloned().collect();
                    let renamed = crate::merge::free_key(&key, &taken);
                    warnings.push(Warning {
                        source: source_of(at),
                        page: None,
                        detail: format!(
                            "§14.7.6.2: the attribute class /{} is already this document's with \
                             other attributes, so this source's became /{}; every carried \
                             element's /C was rewritten to match",
                            String::from_utf8_lossy(&key).escape_debug(),
                            String::from_utf8_lossy(&renamed).escape_debug()
                        ),
                    });
                    self.class_renames
                        .entry(at)
                        .or_default()
                        .insert(key, renamed.clone());
                    self.class_map.insert(renamed, carried);
                } else {
                    self.class_map.insert(key, carried);
                }
            }
        }

        for entry in LIST_VALUED {
            let items = {
                let Some(document) = host.source(at) else {
                    return;
                };
                let Some(Object::Array(items)) = root.get(entry).map(|v| document.resolve(v))
                else {
                    continue;
                };
                items
            };
            let carried: Vec<Object> = items
                .iter()
                .map(|item| host.carry_value(at, item))
                .collect();
            self.lists.entry(entry).or_default().extend(carried);
        }

        // §14.7.5.4 makes `/ParentTreeNextKey` "an integer value greater than any that is
        // currently in use as a key in the structural parent tree" — a statement about the file
        // that holds the tree, so the output states its own rather than any source's.
        if root.get("ParentTreeNextKey").is_some() {
            warnings.push(Warning {
                source: source_of(at),
                page: None,
                detail: "§14.7.5.4: this source's /ParentTreeNextKey is not carried; the entry \
                         names a key in the parent tree of the file that states it, and the \
                         output states its own"
                    .to_owned(),
            });
        }
    }

    /// Table 354's `/IDTree`, and the collision Table 355 forbids.
    ///
    /// # Errors
    ///
    /// [`Refusal::StructureConflict`] where two kept elements state one `/ID`.
    pub(super) fn check_ids(
        &mut self,
        host: &mut dyn Host,
        source_of: &dyn Fn(usize) -> usize,
    ) -> Result<(), Refusal> {
        let mut collisions: Vec<String> = Vec::new();
        // One `/ID` may be stated by two elements of one source — the clause binds the document
        // that already held both, and carrying what the producer wrote is RFC 0002 section 11.1's
        // premise, which is ADR 0821 section 2's distinction. What cannot be written is one `/ID`
        // stated by two *different* sources, because the derived hierarchy holds both elements.
        for element in &self.elements {
            let Some(document) = host.source(element.at) else {
                continue;
            };
            let Some(Object::String(bytes)) = document.get_key_of(element.source, "ID") else {
                continue;
            };
            let Some((first, _)) = self.ids.get(bytes.as_ref()) else {
                continue;
            };
            if *first != element.at {
                collisions.push(format!(
                    "/ID {} (sources {} and {})",
                    String::from_utf8_lossy(&bytes).escape_debug(),
                    source_of(*first),
                    source_of(element.at)
                ));
            }
        }
        collisions.sort();
        collisions.dedup();
        if collisions.is_empty() {
            return Ok(());
        }
        Err(Refusal::StructureConflict {
            clause: "Table 355 makes a structure element's /ID \"unique among all elements in \
                     the document's structure hierarchy\", and the derived document is one \
                     hierarchy",
            keys: collisions.join("; "),
        })
    }

    /// Table 353's flags, read off every contributing document.
    ///
    /// §14.7.1's mark information dictionary, whose `/Marked` row reads:
    ///
    /// > A flag indicating whether the document conforms to tagged PDF conventions
    ///
    /// A document some of whose pages came out of a source that stated no structure does not
    /// conform to them, whatever its other sources said, so `/Marked` is a conjunction. The other
    /// two flags state that something is *present*, so they are unions.
    pub(super) fn read_mark_info(&mut self, host: &mut dyn Host, contributing: &[usize]) {
        for at in contributing {
            let flags = host.source(*at).and_then(|document| {
                let catalog = document.catalog().ok()?;
                match catalog.get("MarkInfo").map(|v| document.resolve(v)) {
                    Some(Object::Dictionary(dict)) => Some(dict),
                    _ => None,
                }
            });
            let flag = |key: &str| {
                matches!(
                    flags.as_ref().and_then(|dict| dict.get(key)),
                    Some(&Object::Boolean(true))
                )
            };
            if !flag("Marked") {
                self.marked = false;
            }
            self.user_properties |= flag("UserProperties");
            self.suspects |= flag("Suspects");
        }
    }
}
