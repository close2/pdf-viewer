//! ISO 32000-2 §8.10.4's reference `XObject`s: what a proxy names, and who supplies it.
//!
//! # The two processors the clause addresses
//!
//! §8.10.4.1 writes one sentence to each of two classes, and both are `shall`s:
//!
//! > PDF processors that do not recognise the Ref entry shall simply display or print the proxy
//! > as an ordinary form XObject. Those PDF processors that do implement reference XObjects
//! > shall use the proxy in place of the imported content if the latter is unavailable.
//!
//! This tree was the first class for its whole life, which is conforming and was recorded as
//! such. What moved it is not the conformance clause but `CLAUDE.md`'s "done": a reachable
//! target page is content a producer specified, and drawing a grey box in its place is content
//! lost. So this module is the second class, under the one constraint principle 3 imposes —
//! **the renderer has no filesystem**, so the target document is not fetched here. It is
//! *supplied*, by a host, exactly as a trust store is (ADR 1076), and ADR 1101 is the decision.
//!
//! # Why the match is on §14.4's identifier and never on a path
//!
//! Table 95 makes `/F` — a file specification — required, and [`crate::file_spec`] is explicit
//! that a specification is "read whole and opened never". A path is a sentence about somebody
//! else's filesystem; this program has none to compare it against, and matching on a name would
//! make *which file is imported* a function of a string a document chose. §8.10.4.1 Table 95's
//! optional `/ID` is what the clause offers instead:
//!
//! > An array of two byte strings constituting a PDF file identifier (14.4, "File identifiers")
//! > for the PDF file containing the target document. The use of this entry improves a PDF
//! > processor's chances of finding the intended PDF file and allows it to warn the user if the
//! > PDF file has changed since the reference was created.
//!
//! And §14.4 does not merely say what the two strings are — it states the match, in a sentence
//! whose subject is a *reference*:
//!
//! > If the first identifier in the reference matches the first identifier in the referenced
//! > file's ID entry, and the last identifier in the reference matches the last identifier in the
//! > referenced file's ID entry, it is very likely that the correct and unchanged PDF file has
//! > been found. If only the first identifier matches, a different version of the correct PDF
//! > file has been found.
//!
//! So the rule is the standard's and not this project's: the permanent string decides *which
//! file*, and a difference in the changing string is "a different version of the correct PDF
//! file" — §8.10.4.1's warning condition, and a thing to say rather than a thing to refuse.
//!
//! A reference stating no `/ID` therefore imports nothing. That is a deliberate refusal and it is
//! said out loud wherever a host actually supplied files ([`Outcome::NotIdentified`]) — trap 5's
//! rule, since the alternative is a reader that silently picks one of the files it was given.

use pdf_syntax::{Dictionary, Document, Object};

use crate::file_spec::FileSpec;

/// How many pages of a supplied document are walked to resolve Table 95's page *label*.
///
/// §12.4.2's labels are not indexed by value — [`crate::page_label::PageLabels`] answers
/// index → label — so a label is resolved by walking. The bound is on the walk rather than on
/// the document: a reference naming a label no early page carries falls back to the proxy, which
/// is §8.10.4.1's own answer for content that is unavailable. It is far past any real document's
/// front matter and exists so that one `Do` cannot cost a walk of a ten-thousand-page target.
const MAX_LABELS_SEARCHED: usize = 4096;

/// Table 95's `/Page`: "[a] page index or page label … identifying the page of the target
/// document containing the content to be imported".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Designation {
    /// An integer, which §12.4.2 makes zero-based: "the page index of the first page in a
    /// labelling range" counts from the first page of the document, which is index 0.
    Index(usize),
    /// A text string, matched against §12.4.2's label for each page in turn.
    Label(String),
}

/// ISO 32000-2 §8.10.4.1 Table 95's reference dictionary.
///
/// Read from a form `XObject`'s `/Ref`, which §8.10.4.1 makes the thing that distinguishes a
/// reference `XObject` from any other form: "[t]he presence of the Ref entry shall distinguish
/// reference `XObjects` from other types of form `XObjects`."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// Table 95's `/F`, "[t]he PDF file containing the target document" (required).
    pub file: FileSpec,
    /// Table 95's `/Page` (required).
    pub page: Designation,
    /// Table 95's `/ID`, §14.4's two-string identifier of the target file (optional).
    pub id: Option<[Vec<u8>; 2]>,
}

impl Reference {
    /// Reads a form dictionary's `/Ref`, or `None` where the form states none.
    ///
    /// `None` also for a `/Ref` that states neither of Table 95's two required entries in a
    /// readable form — such a dictionary names no file and no page, so there is nothing for a
    /// supply to be asked about and §8.10.4.1's proxy is drawn with nothing said. It is the
    /// *presence* of the entry that makes a form a reference `XObject`, and a malformed one is
    /// still a reference `XObject`; what it is not is a reference to anything.
    #[must_use]
    pub fn read(document: &Document, form: &Dictionary) -> Option<Self> {
        let entry = document.get_key(form, "Ref");
        let dict = entry.as_dict()?;
        let file = FileSpec::parse(document, &document.get_key(dict, "F"))?;
        let page = match document.get_key(dict, "Page") {
            Object::Integer(index) => Designation::Index(usize::try_from(index).ok()?),
            Object::String(bytes) => Designation::Label(pdf_syntax::text_string(&bytes)),
            _ => return None,
        };
        Some(Self {
            file,
            page,
            id: identifier(&document.get_key(dict, "ID")),
        })
    }
}

/// §14.4's file identifier as an object states it: an array of two byte strings.
fn identifier(object: &Object) -> Option<[Vec<u8>; 2]> {
    let items = object.as_array()?;
    match (items.first(), items.get(1)) {
        (Some(Object::String(first)), Some(Object::String(second))) => {
            Some([first.to_vec(), second.to_vec()])
        }
        _ => None,
    }
}

/// One target document a host has supplied, under the name the host knows it by.
#[derive(Debug)]
pub struct Supplied {
    /// What the host called it. Never matched against — it is what a refusal or a report says.
    name: String,
    /// §14.4's identifier out of this file's own trailer. Every held file has one: a file
    /// without it is refused at [`Supply::read`], because nothing could name it.
    identifier: [Vec<u8>; 2],
    /// The parsed file.
    document: Document,
}

impl Supplied {
    /// The name the host knows this file by.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The parsed target document.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }
}

/// A file a host offered that this reader will not import from, named rather than counted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The bytes are not a PDF this reader can open.
    #[error("{name} is not a PDF file this reader can open: {error}")]
    Unopenable {
        /// The name the host offered it under.
        name: String,
        /// What `pdf-syntax` said.
        error: String,
    },
    /// The file opens but states no §14.4 identifier, so no `/Ref` can name it.
    ///
    /// §7.5.5 Table 15 makes the entry "Required in PDF 2.0 and later, or if an Encrypt entry is
    /// present; optional otherwise", so a file without one is either pre-2.0 or malformed — a
    /// statement about the file rather than about this reader. It is refused rather than kept
    /// because §14.4's match has nothing to run against it, and a file nothing can name is a file
    /// this reader would hold and never use.
    #[error("{name} states no /ID in its trailer, so no reference dictionary can identify it")]
    Unidentified {
        /// The name the host offered it under.
        name: String,
    },
}

/// The target documents a host has supplied, and where it says they came from.
///
/// **[`Supply::none`] is the default and is what every host has until somebody says otherwise.**
/// With no supply, every reference `XObject` draws §8.10.4.1's proxy and reports nothing, which
/// is what this tree did for its whole life and what the clause writes for a processor in that
/// position.
#[derive(Debug, Default)]
pub struct Supply {
    documents: Vec<Supplied>,
    /// The sentence saying where these came from, for a report to quote.
    ///
    /// ADR 1076's reason, one construct over: a reader told that a page came from another file
    /// is owed *which other file, on whose say-so*, and this program did not choose it.
    source: String,
}

impl Supply {
    /// No target documents at all, as a constant a default caller can borrow.
    ///
    /// `const` rather than a function so that [`crate::content::interpret_with_fonts`] has
    /// something with the whole program's lifetime to hand [`crate::content::interpret_importing`]
    /// — which is what keeps the two from being two implementations of one interpretation.
    pub const NONE: Self = Self {
        documents: Vec::new(),
        source: String::new(),
    };

    /// No target documents at all.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Takes the files a host read, parsing each once.
    ///
    /// `source` is the sentence saying where they came from — a directory a person named, the
    /// word a host was given — and it is carried rather than derived, because this crate has no
    /// filesystem to describe.
    ///
    /// Each file is opened here rather than at the `Do` that needs it, for two reasons that both
    /// point the same way: a parse failure is a fact about the *host's* input and belongs in the
    /// refusal list a host prints once, and a document parsed per `Do` would make one page's
    /// cost a function of how many times it names a reference.
    #[must_use]
    pub fn read<I, N>(files: I, source: impl Into<String>) -> (Self, Vec<Refusal>)
    where
        I: IntoIterator<Item = (N, Vec<u8>)>,
        N: Into<String>,
    {
        let mut documents = Vec::new();
        let mut refused = Vec::new();
        for (name, bytes) in files {
            let name = name.into();
            match Document::open(bytes) {
                Err(error) => refused.push(Refusal::Unopenable {
                    name,
                    error: error.to_string(),
                }),
                Ok(document) => {
                    let stated = document
                        .trailer()
                        .get("ID")
                        .map_or(Object::Null, |entry| document.resolve(entry));
                    let Some(identifier) = identifier(&stated) else {
                        refused.push(Refusal::Unidentified { name });
                        continue;
                    };
                    documents.push(Supplied {
                        name,
                        identifier,
                        document,
                    });
                }
            }
        }
        (
            Self {
                documents,
                source: source.into(),
            },
            refused,
        )
    }

    /// Whether a host supplied anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// How many target documents were supplied.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// The sentence saying where these came from.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Which supplied file, if any, Table 95's `/ID` names — and which page of it.
    ///
    /// The match is §14.4's pair and nothing else, in the order that clause states the two
    /// strings in. See this module's header for why a path is not consulted.
    #[must_use]
    pub fn find(&self, reference: &Reference) -> Outcome<'_> {
        if self.documents.is_empty() {
            return Outcome::NoneSupplied;
        }
        let Some([permanent, changing]) = reference.id.as_ref() else {
            return Outcome::NotIdentified;
        };
        let Some(found) = self
            .documents
            .iter()
            .find(|supplied| &supplied.identifier[0] == permanent)
        else {
            return Outcome::Unavailable;
        };
        let Some(index) = page_index(&found.document, &reference.page) else {
            return Outcome::NoSuchPage {
                supplied: found,
                changed: changed(found, changing),
            };
        };
        Outcome::Found {
            supplied: found,
            index,
            changed: changed(found, changing),
        }
    }
}

/// Whether §14.4's *changing* identifier differs from what the reference recorded.
///
/// > If only the first identifier matches, a different version of the correct PDF file has been
/// > found.
fn changed(supplied: &Supplied, changing: &[u8]) -> bool {
    supplied.identifier[1] != changing
}

/// Table 95's `/Page` resolved against a target document, or `None` where it names no page.
fn page_index(document: &Document, page: &Designation) -> Option<usize> {
    let pages = crate::page::Pages::new(document);
    match page {
        Designation::Index(index) => (*index < pages.len()).then_some(*index),
        Designation::Label(label) => {
            let labels = crate::page_label::PageLabels::read(document);
            (0..pages.len().min(MAX_LABELS_SEARCHED))
                .find(|index| labels.label(*index).as_deref() == Some(label.as_str()))
        }
    }
}

/// What a [`Supply`] answers about one reference dictionary.
///
/// Five answers rather than two, because §8.10.4.1 draws the proxy in four of them and a reader
/// that could not say *which* four would be a reader nobody can ask why a page is grey.
#[derive(Debug)]
pub enum Outcome<'a> {
    /// The page is here. §8.10.4.1's second class draws it.
    Found {
        /// The file it is in.
        supplied: &'a Supplied,
        /// Its zero-based index in that file.
        index: usize,
        /// Whether §14.4's changing identifier differs — the clause's own warning condition.
        changed: bool,
    },
    /// No host supplied anything, so the proxy is drawn and nothing is said.
    ///
    /// This is §8.10.4.1's sentence about content that "is unavailable" and it is the state
    /// every host is in by default. It is *not* reported: the clause states this alternative,
    /// so there is no gap to name (trap 11).
    NoneSupplied,
    /// Files were supplied and this reference states no `/ID`, so none of them can be named.
    NotIdentified,
    /// Files were supplied and none carries §14.4's permanent identifier this reference records.
    Unavailable,
    /// The named file is here and Table 95's `/Page` names no page of it.
    ///
    /// Table 95 says this can happen without anybody doing anything wrong: "[t]his reference is a
    /// weak one and may be inadvertently invalidated if the referenced page is changed or
    /// replaced in the target document after the reference is created."
    NoSuchPage {
        /// The file it named.
        supplied: &'a Supplied,
        /// Whether §14.4's changing identifier differs as well.
        changed: bool,
    },
}
