//! What the source's encryption asserted, kept as a statement once it can no longer be enforced.
//!
//! ISO 19005-2 section 6.1.3 and ISO 19005-4 section 6.1.3 forbid an `/Encrypt` key in the
//! trailer, so no output of this verb is encrypted and no output of this verb can enforce
//! ISO 32000-2 §7.6.4.2's Table 22 permission flags. **The enforcement cannot survive; the
//! assertion can**, and `doc/pdf-a-mitigations.md` section 2 is where that `preserve` is named
//! beside the `discard` the removal is. What an archivist wants to know twenty years from now is
//! what the document *claimed* about its reader, and this module is what puts that in the report
//! and in the output's own `xmpMM:History` before the claim goes.
//!
//! Reading, not deciding. `doc/adr/1187` is the argument; nothing here rewrites anything.

use pdf_syntax::Document;

/// What the source's encryption dictionary asserted about the person reading the document.
///
/// Every field is read from the source and none is computed: a report a person acts on has to be
/// able to say *the file said this*, and a converter that reworded Table 22 into its own scheme
/// would be asserting rather than carrying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProtection {
    /// §7.6.1's `/Filter`: the security handler the producer named.
    pub handler: String,
    /// Table 20's `/V`, the algorithm the handler used, where the dictionary states one.
    pub algorithm: Option<i64>,
    /// Table 21's `/R`, the handler's revision — which is what decides Table 22's meanings.
    pub revision: u8,
    /// Every Table 22 position this program reads, in the clause's own order, with what the
    /// source's flag word said about it.
    ///
    /// Each pair is the operation in the clause's words and whether the producer permitted it.
    pub permissions: Vec<(&'static str, bool)>,
    /// Whether the source was opened under §7.6.4.1's owner password, which "should allow full
    /// (owner) access" and makes every flag above advisory for this reader.
    pub as_owner: bool,
}

/// Table 22's positions, in the order the clause prints them, each with the operation it governs.
///
/// The words are the clause's own, shortened to the operation rather than the sentence, because a
/// report line is read beside forty others. Bit 10 is absent on purpose: Table 22 says a reader
/// "shall ignore this bit", so a file's value for it asserts nothing.
const OPERATIONS: [&str; 7] = [
    "print the document (bit 3)",
    "modify the contents (bit 4)",
    "copy or extract text and graphics (bit 5)",
    "add or modify annotations and fill in form fields (bit 6)",
    "fill in existing form fields (bit 9)",
    "assemble the document (bit 11)",
    "print at full fidelity (bit 12)",
];

impl SourceProtection {
    /// What this source's `/Encrypt` dictionary asserts, or `None` where it carries none.
    ///
    /// The flag word is `pdf_syntax`'s reading of §7.6.4.2 rather than a second one taken here:
    /// which bits are meaningful depends on the handler's revision, and that reading has one home.
    #[must_use]
    pub fn of(document: &Document) -> Option<Self> {
        let permissions = document.permissions()?;
        let encrypt = document.trailer().get("Encrypt")?;
        let encrypt = document.resolve(encrypt);
        let dict = encrypt.as_dict();
        let handler = dict
            .and_then(|dict| document.get_key(dict, "Filter").as_name().cloned())
            .map_or_else(
                || "unnamed".to_owned(),
                |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
            );
        let algorithm = dict.and_then(|dict| document.get_key(dict, "V").as_integer());
        let granted = [
            permissions.print,
            permissions.modify,
            permissions.copy,
            permissions.annotate,
            permissions.fill_forms,
            permissions.assemble,
            permissions.print_faithfully,
        ];
        Some(Self {
            handler,
            algorithm,
            revision: permissions.revision,
            permissions: OPERATIONS.into_iter().zip(granted).collect(),
            as_owner: permissions.owner,
        })
    }

    /// The operations the producer's flag word withheld, in the clause's order.
    #[must_use]
    pub fn withheld(&self) -> Vec<&'static str> {
        self.permissions
            .iter()
            .filter_map(|&(operation, granted)| (!granted).then_some(operation))
            .collect()
    }

    /// The sentence the report carries, saying what the source claimed and what the output cannot.
    #[must_use]
    pub fn sentence(&self) -> String {
        let withheld = self.withheld();
        let asserted = if withheld.is_empty() {
            "withheld no operation this program reads".to_owned()
        } else {
            format!("withheld {}", withheld.join(", "))
        };
        format!(
            "the source was encrypted under the {} handler (V {}, R {}) and {asserted}. The \
             output carries no encryption, so it is readable by anybody holding it and asserts \
             none of these flags",
            self.handler,
            self.algorithm
                .map_or_else(|| "unstated".to_owned(), |value| value.to_string()),
            self.revision,
        )
    }

    /// The `xmpMM:History` parameters recording what the source asserted.
    ///
    /// `doc/pdf-a-mitigations.md` section 2's `preserve`: the fact belongs in the *file*, not only
    /// in a report somebody may not have kept, for the reason `doc/questions/A55` gives one remedy
    /// over — an archive that lost the claim silently has lost what an archivist came for.
    #[must_use]
    pub fn history(&self) -> String {
        format!("{PERMISSIONS_NO_LONGER_ASSERTED}: {}", self.sentence())
    }
}

/// The sentence a report carries in front of a removed encryption's permission statement.
pub const PERMISSIONS_NO_LONGER_ASSERTED: &str =
    "this document's permission flags are recorded, not enforced";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_with_no_encryption_has_nothing_to_state() {
        let document = Document::empty();
        assert!(SourceProtection::of(&document).is_none());
    }

    #[test]
    fn the_sentence_names_every_operation_the_flag_word_withheld() {
        let protection = SourceProtection {
            handler: "Standard".to_owned(),
            algorithm: Some(2),
            revision: 3,
            permissions: OPERATIONS
                .into_iter()
                .zip([false, true, false, true, true, true, true])
                .collect(),
            as_owner: false,
        };
        let sentence = protection.sentence();
        assert!(
            sentence.contains("print the document (bit 3)"),
            "{sentence}"
        );
        assert!(
            sentence.contains("copy or extract text and graphics (bit 5)"),
            "{sentence}"
        );
        assert!(!sentence.contains("bit 4"), "{sentence}");
        assert_eq!(protection.withheld().len(), 2);
    }

    #[test]
    fn a_flag_word_that_withholds_nothing_says_so() {
        let protection = SourceProtection {
            handler: "Standard".to_owned(),
            algorithm: None,
            revision: 2,
            permissions: OPERATIONS.into_iter().zip([true; 7]).collect(),
            as_owner: true,
        };
        assert!(protection.withheld().is_empty());
        assert!(
            protection.sentence().contains("withheld no operation"),
            "{}",
            protection.sentence()
        );
        assert!(protection.sentence().contains("V unstated"));
    }
}
