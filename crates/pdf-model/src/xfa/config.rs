//! XFA 3.3's `config` packet, read for what binding a field's data and validating a form at a save
//! depend on.
//!
//! Three things are read, each where XFA 3.3 places it (ADR 2450):
//!
//! - **Chapter 15's extended mapping rules** (pages 508–509): the `data` element of a `common`
//!   element. Its `transform` elements' `picture` is sequence 14 of page 509's table, the default
//!   bind picture page 527 gives a field whose template states none, for the data elements the
//!   transform's `ref` names. The table's other rules — sequences 1 to 13 — change which data
//!   nodes exist, what they are named and what they hold, so this program, which carries none of
//!   them out, names them rather than binding past them (trap 5).
//! - **Page 1340's `validate`**: the activities before which validations run; `preSave` is the
//!   one that makes a save a moment the null test is run at, and the default is `preSubmit` alone.
//!   Page 1340's note has the Acrobat family look under `acrobat` and fall back to `present`, and
//!   this program, a PDF viewer reading the PDF form, does the same.
//! - **Page 386's `validationMessaging`**: how the messages of a validation are presented.
//!
//! **Every `common` element's `data` is read.** XFA 3.3 places `common` under each application's
//! element (`present`, `acrobat`, …) and says nothing of which one's `data` a reader takes; a rule
//! stated in any of them is a rule some processor carries out over this form's data, so a rule
//! this program does not follow is named wherever it is stated. That is a choice, and the locale
//! reader's first `common` with a `locale` is its precedent (ADR 2414). XFA 3.3 is cited and
//! paraphrased, never quoted (RFC 0011's preamble).

use super::xml::Tree;

/// How a form's validation messages are presented: page 386's `validationMessaging`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Messaging {
    /// Each message on its own: page 386's default.
    #[default]
    Individually,
    /// Every message in one.
    Together,
    /// The first message alone, the others suppressed while their content stays invalid.
    FirstOnly,
    /// No message, the invalid content staying invalid.
    None,
}

/// What a form's `config` packet states about its data and its validations.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Each `transform`'s `ref` and the default bind picture its `picture` gives, in document
    /// order (page 527).
    pictures: Vec<(String, String)>,
    /// The extended mapping rules the `data` elements state that this program does not carry
    /// out, by element name (pages 508–509).
    unfollowed: Vec<String>,
    /// Whether validations run before a form is saved to a file (page 1340's `preSave`).
    validates_on_save: bool,
    /// Page 386's option.
    messaging: Messaging,
}

impl Config {
    /// Reads the `config` element of `tree`; the defaults where `tree` holds none.
    pub(crate) fn read(tree: &Tree) -> Self {
        let mut config = Self::default();
        let Some(root) = tree.first("config") else {
            return config;
        };
        let applications: Vec<usize> = tree
            .get(root)
            .map(|element| element.children.clone())
            .unwrap_or_default();
        for &application in &applications {
            let Some(common) = tree.child(application, "common") else {
                continue;
            };
            if let Some(data) = tree.child(common, "data") {
                config.read_data(tree, data);
            }
            if let Some(text) = tree
                .child(common, "validationMessaging")
                .and_then(|at| tree.get(at))
            {
                config.messaging = match text.text.trim() {
                    "allMessagesTogether" => Messaging::Together,
                    "firstMessageOnly" => Messaging::FirstOnly,
                    "noMessages" => Messaging::None,
                    _ => Messaging::Individually,
                };
            }
        }
        // Page 1340's note: the Acrobat family reads `acrobat`'s `validate`, and `present`'s
        // only where `acrobat` states none. An element stated empty validates before nothing.
        let validate = ["acrobat", "present"].iter().find_map(|name| {
            let application = tree.child(root, name)?;
            tree.get(tree.child(application, "validate")?)
        });
        config.validates_on_save =
            validate.is_some_and(|element| element.text.split_whitespace().any(|a| a == "preSave"));
        config
    }

    /// One `data` element's rules: each `transform`'s picture, and the name of every rule of page
    /// 509's table this program does not carry out.
    fn read_data(&mut self, tree: &Tree, data: usize) {
        let Some(element) = tree.get(data) else {
            return;
        };
        for &child in &element.children {
            let Some(rule) = tree.get(child) else {
                continue;
            };
            match rule.local.as_str() {
                // Sequences 1, 2, 3, 12 and 13 of page 509's table: the start node, the excluded
                // namespaces, attributes as data values, the record elements and the record range.
                "startNode" | "excludeNS" | "attributes" | "record" | "range" => {
                    self.unfollowed.push(rule.local.clone());
                }
                // Page 545's XSLT pre- and postprocessing; page 898 makes an empty `uri` mean
                // the processing is not done.
                "xsl" | "outputXSL" => {
                    let stated = tree
                        .child(child, "uri")
                        .and_then(|uri| tree.get(uri))
                        .is_some_and(|uri| !uri.text.trim().is_empty());
                    if stated {
                        self.unfollowed.push(rule.local.clone());
                    }
                }
                "transform" => {
                    for &part in &rule.children {
                        let Some(part) = tree.get(part) else {
                            continue;
                        };
                        if part.local == "picture" {
                            let picture = part.text.trim();
                            if let (Some(reference), false) =
                                (rule.attribute("ref"), picture.is_empty())
                            {
                                self.pictures
                                    .push((reference.to_owned(), picture.to_owned()));
                            }
                        } else {
                            // Sequences 4 and 6 to 11: presence, rename, nameAttr, whitespace,
                            // ifEmpty and groupParent.
                            self.unfollowed.push(format!("transform's {}", part.local));
                        }
                    }
                }
                // `adjustData`, `incrementalLoad`, `window` and `uri` govern when and from where
                // data is loaded and how it is re-normalised after a merge, not how an element
                // of it maps, so none of them changes the node a field binds.
                _ => {}
            }
        }
    }

    /// The sentence naming the extended mapping rules this program does not carry out, where
    /// the packet states any.
    pub(crate) fn unfollowed(&self) -> Option<String> {
        (!self.unfollowed.is_empty()).then(|| {
            format!(
                "its XFA config packet states extended mapping rules (XFA 3.3 chapter 15, pages \
                 508–509) this program does not carry out — {} — so the data node it binds is \
                 not known here",
                self.unfollowed.join(", ")
            )
        })
    }

    /// The default bind picture page 527 gives a data element named `name`; `None` where no
    /// `transform` names it.
    ///
    /// # Errors
    ///
    /// Two `transform`s naming it with different pictures: the element's reference says that
    /// overlapping transforms are order-sensitive and recommends against them, and states nothing
    /// that orders two pictures, so this program names the field rather than choosing one.
    pub(crate) fn bind_picture(&self, name: &str) -> Result<Option<&str>, String> {
        let mut named = self
            .pictures
            .iter()
            .filter(|(reference, _)| reference == name)
            .map(|(_, picture)| picture.as_str());
        let Some(first) = named.next() else {
            return Ok(None);
        };
        if named.any(|other| other != first) {
            return Err(format!(
                "its XFA config packet gives the data element {name:?} more than one default \
                 bind picture (XFA 3.3 page 527), and nothing there orders them"
            ));
        }
        Ok(Some(first))
    }

    /// Whether the form asks for its validations before a save (page 1340's `preSave`).
    pub(crate) fn validates_on_save(&self) -> bool {
        self.validates_on_save
    }

    /// Page 386's option.
    pub(crate) fn messaging(&self) -> Messaging {
        self.messaging
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Messaging};
    use crate::xfa::xml::parse;

    fn config(text: &str) -> Config {
        Config::read(&parse(text.as_bytes()).expect("well formed"))
    }

    #[test]
    fn the_four_hybrids_configuration_states_no_rule_this_does_not_follow() {
        // The `data` element all four of RFC 0011 section 4's hybrids carry, and their
        // `validate`: no transform, empty XSLT URIs, validation before submitting alone.
        let read = config(
            "<config><acrobat><common><data><adjustData/><xsl><uri/></xsl><outputXSL><uri/>\
             </outputXSL></data></common><validate>preSubmit</validate></acrobat></config>",
        );
        assert_eq!(read.unfollowed(), None);
        assert!(!read.validates_on_save());
        assert_eq!(read.bind_picture("prix"), Ok(None));
        assert_eq!(read.messaging(), Messaging::Individually);
    }

    #[test]
    fn a_transform_gives_its_ref_a_default_bind_picture() {
        // XFA 3.3 EXAMPLE 15.24's transform.
        let read = config(
            "<config><present><common><data><transform ref=\"prix\"><picture>$999.99</picture>\
             </transform></data><validationMessaging>allMessagesTogether</validationMessaging>\
             </common><validate>preSubmit preSave</validate></present></config>",
        );
        assert_eq!(read.bind_picture("prix"), Ok(Some("$999.99")));
        assert_eq!(read.bind_picture("price"), Ok(None));
        assert!(read.validates_on_save());
        assert_eq!(read.messaging(), Messaging::Together);
    }

    #[test]
    fn acrobat_s_validate_wins_over_present_s() {
        let read = config(
            "<config><present><validate>preSave</validate></present><acrobat><validate/>\
             </acrobat></config>",
        );
        assert!(!read.validates_on_save());
    }

    #[test]
    fn a_rule_this_does_not_follow_is_named() {
        let read = config(
            "<config><acrobat><common><data><startNode>x</startNode><xsl><uri>a.xsl</uri></xsl>\
             <transform ref=\"a\"><rename>b</rename><picture>9</picture></transform>\
             <transform ref=\"a\"><picture>99</picture></transform></data></common></acrobat>\
             </config>",
        );
        let said = read.unfollowed().expect("rules are stated");
        assert!(
            said.contains("startNode, xsl, transform's rename"),
            "{said}"
        );
        assert!(read.bind_picture("a").is_err());
    }
}
