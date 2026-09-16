//! ISO 32000-2 §7.11's file specifications: what a document says a file is, and never opens.
//!
//! # Why a reader with no filesystem reads these at all
//!
//! §7.11.1 is unambiguous about what a file specification refers to:
//!
//! > A file specification shall refer to a file external to the PDF file or to a file embedded
//! > within the referring PDF file, allowing its contents to be stored or transmitted along with
//! > the PDF file. The file is considered external to the PDF file in either case.
//!
//! `CLAUDE.md` principle 3 gives the renderer no filesystem and no network, so *following* one is
//! refused by architecture. What is left is not nothing: a specification is a statement the
//! document makes, and a viewer that lists attachments, names the target of a link, or says why
//! it declined has to read the statement before it can decline it. Every place in this tree that
//! puts a file's *name* in front of a person reads it out of one of these: §12.5.6.15's file
//! attachment annotation, §7.11.4's embedded files and §14.13's associated ones, §12.6.4.4's
//! embedded go-to, and §12.7.6.4's import-data action.
//!
//! **That list said something stronger and false until the eight-hundred-and-fifty-third
//! session** — "[e]very refusal in this tree that names a file — §7.3.8's external stream data,
//! §8.10.4's reference `XObject`, §12.6.4.6's launch action, §12.6.4.4's embedded go-to" — and
//! three of its four members name no file at all. §7.3.8's external stream data is a bare
//! [`pdf_syntax::StreamRefusal::External`], decided in a crate that cannot depend on this one;
//! §12.6.4.6's launch is a fixed sentence, "Launch: running an application, which the sandbox
//! withholds", as `GoToR`'s and `Thread`'s are; and §8.10.4's reference `XObject` reads nothing —
//! nothing in `crates/`, `tools/` or `fuzz/` names `/Ref`, which is §8.10.4.1's own provision for
//! a processor that draws the proxy instead ([`crate::content`], and
//! `pdf-model/tests/reference_xobjects.rs` holds it). What would make the retired sentence true is
//! a refusal carrying an owned string rather than a `&'static str`; that is a change to
//! [`crate::action`]'s boundary and not to this module, and it is not owed by any clause.
//!
//! # What this module is not
//!
//! It never produces a path for an operating system. §7.11.2.1 ends with the sentence that makes
//! that a design constraint rather than a limitation:
//!
//! > The component substrings shall be stored as bytes and shall be passed to the operating
//! > system without interpretation or conversion of any sort.
//!
//! So [`FileSpec::components`] are `Vec<u8>` and stay that way. Table 43 makes `/UF` a *text
//! string* (§7.9.2.2) and `/F` a byte string, and the difference is real: three call sites in
//! this crate used to decode `/F` with `text_string` and hand the result on as a name, which
//! corrupts a file name in every locale whose bytes are not UTF-8 or `PDFDocEncoding`.
//! [`FileSpec::display_name`] is the one place a byte string becomes text, and it says so.

use pdf_syntax::{Dictionary, Document, Object};

/// How many components a file specification string may hold.
///
/// A path is not a data structure a document should be able to make arbitrarily large; the
/// bound is far past any real one and exists because the split is linear in the string.
const MAX_COMPONENTS: usize = 256;

/// §7.11.3's `/FS`, "[t]he name of the file system that shall be used to interpret this file
/// specification".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FileSystem {
    /// No `/FS`: the specification is a path in §7.11.2's standard format.
    #[default]
    Standard,
    /// `/FS /URL`, which §7.11.5 makes `/F` "a uniform resource locator (URL) of the form
    /// defined in Internet RFC 3986" rather than a path.
    Url,
    /// A name an application registered under Annex E. The clause defines exactly one standard
    /// name, so anything else is a file system this program knows nothing about — and naming it
    /// is the whole of what a reader can do.
    Other(String),
}

/// The encrypted payload dictionary §7.11.3's Table 43 hangs off `/EP`, defined by §7.6.7.
///
/// ISO 32000-2 §7.11.3, Table 43, states what the entry is for:
///
/// > The value of this key is an encrypted payload dictionary which identifies that the file
/// > specified in the EF dictionary is an encrypted payload.
///
/// ISO 32000-2 §7.6.7's construction is a PDF carrying another PDF: a producer using a handler
/// this standard does not define embeds the encrypted document inside an ordinary unencrypted one,
/// so that a reader without the handler still has something to open and something to read.
///
/// > An unencrypted wrapper should provide guidance informing users of the security handler that
/// > is needed to decrypt the embedded encrypted PDF file ( encrypted payload ).
///
/// # What a reader that cannot decrypt it owes, and what it does not
///
/// This program has no cryptographic filter outside §7.6.5's standard one, so the payload is a
/// file it cannot open — and the clause says that case is expected rather than exceptional:
/// "PDF processors without the custom security handler will present the unencrypted wrapper
/// document with helpful instructions to the user", which is what drawing the wrapper's own pages
/// already does. What was missing was the *naming*: until this dictionary was read, an attachment
/// that is an encrypted payload was listed as an ordinary embedded file whose bytes happened to be
/// undecodable, and the one thing the file says about why — which filter would open it — was not
/// read by anybody. [`Self::filter`] is that sentence, and it is why this reader exists without a
/// consumer that decrypts anything.
///
/// Presenting the payload *instead of* the wrapper is §12.3.5's collection, not this clause's, and
/// it stays out of reach for the same reason: there is no handler for what `/Subtype` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPayload {
    /// §7.6.7's Table 28 `/Subtype`, **required**: "[t]he name of the cryptographic filter used
    /// to encrypt the encrypted payload document."
    ///
    /// > This allows a PDF processor to easily determine whether it has the appropriate
    /// > cryptographic filter.
    ///
    /// `None` is a file that stated an encrypted payload dictionary and left out the one entry
    /// that says what it is encrypted with, which is the clause's single `shall` on a producer
    /// here. It is kept as `None` rather than refused, because the dictionary's *presence* is
    /// already the fact a reader needs — this file is a wrapper — and the name is what it cannot
    /// act on either way.
    pub filter: Option<String>,
    /// Table 28's `/Version`: "[t]he version number of the cryptographic filter used to encrypt
    /// the encrypted payload referenced by this dictionary."
    ///
    /// A *name*, and the table's NOTE says how to read it: "[t]he value of Version is not to be
    /// interpreted as a real number but as integers with a PERIOD (2Eh) between them." So it is
    /// carried as the producer spelled it and never parsed into a number.
    pub version: Option<String>,
    /// Whether §7.6.7's Table 28 `/Type` is present and says what the table requires.
    ///
    /// > (Optional) The type of PDF object that this dictionary describes; if present, shall be
    /// > EncryptedPayload for an encrypted payload file specification.
    ///
    /// `true` for an absent entry, because an absent optional entry breaks nothing; `false` is a
    /// document stating a `/Type` the table forbids, which is worth being able to say.
    pub permitted_type: bool,
}

impl EncryptedPayload {
    /// Reads Table 43's `/EP` off a file specification dictionary.
    ///
    /// `None` where the specification states none — which is every file specification in every
    /// corpus on this disk, measured by
    /// `crates/pdf-model/examples/name_dictionary_and_file_spec_census.rs`. The construction is
    /// PDF 2.0's and rare, so a fixture is the witness and the census is what says so.
    #[must_use]
    pub fn read(document: &Document, specification: &Dictionary) -> Option<Self> {
        let payload = document.get_key(specification, "EP");
        let payload = payload.as_dict()?;
        let name = |key: &str| {
            document
                .get_key(payload, key)
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        };
        Some(Self {
            filter: name("Subtype"),
            version: name("Version"),
            permitted_type: name("Type").is_none_or(|stated| stated == "EncryptedPayload"),
        })
    }
}

/// ISO 32000-2 §7.11's file specification, in either of the two forms §7.11.1 gives it.
///
/// A specification is read whole and opened never. `/CI`, `/EF` and `/RF` are read by the modules
/// that own what they point at — [`crate::attachment`] and [`crate::collection`] — and `/EF` is
/// recorded here only as *presence*, because whether a specification carries its file along is
/// the one thing every caller asks. `/Thumb` is [`crate::thumbnail::of_file_spec`]'s, for the
/// same reason and with the same shape: an image is decoded when somebody wants to look at it.
///
/// **That sentence named `/Thumb` and `/EP` among the entries other modules read, and neither was
/// read anywhere** until the eleven-hundred-and-forty-ninth session — the one `/Thumb` reader
/// outside a page's was Table 159's collection flag, which is a boolean. `/EP` is now
/// [`Self::payload`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileSpec {
    /// Table 43's `/FS`.
    pub system: FileSystem,
    /// Table 43's `/UF`, a text string, which §7.11.3 says a reader uses instead of `/F`.
    pub unicode_name: Option<String>,
    /// Table 43's `/F`, kept as the bytes §7.11.2.1 requires.
    ///
    /// Where `system` is [`FileSystem::Url`] this is a URL rather than a path, and §7.11.5 says
    /// it is already a text string, because 7-bit US-ASCII is a strict subset of
    /// `PDFDocEncoding`.
    pub bytes: Option<Vec<u8>>,
    /// Table 43's `/Desc`, "[d]escriptive text associated with the file specification".
    pub description: Option<String>,
    /// Table 43's `/ID`, §14.4's two-string file identifier of the file being referred to.
    pub id: Option<[Vec<u8>; 2]>,
    /// Table 43's `/V`: the file "changes frequently with time", so a reader shall not cache it.
    ///
    /// Nothing here caches anything it refuses to fetch. It is read because a `true` is a
    /// statement about the *target* that a viewer offering to open one has to honour.
    pub volatile: bool,
    /// Whether an `/EF` dictionary is present, which is what makes a specification an
    /// attachment rather than a reference to something outside the document.
    pub embedded: bool,
    /// Table 43's `/EP`, §7.6.7's encrypted payload dictionary, where the file states one.
    pub payload: Option<EncryptedPayload>,
    /// Whether one of Table 43's three deprecated platform keys supplied `bytes`.
    ///
    /// `/DOS`, `/Mac` and `/Unix` are "deprecated in PDF 2.0" and are read only where neither
    /// `/UF` nor `/F` is present, which is the order Table 43's own requirement implies: `/F` is
    /// "[r]equired if the DOS, Mac, and Unix entries are all absent".
    pub platform_specific: bool,
}

impl FileSpec {
    /// Reads §7.11.1's two forms: a string, or a dictionary.
    ///
    /// > A simple file specification shall give just the name of the target file in a standard
    /// > format … It shall take the form of either a string or a dictionary.
    ///
    /// `None` for anything else, which is a document stating something that is not a file
    /// specification at all.
    #[must_use]
    pub fn parse(document: &Document, object: &Object) -> Option<Self> {
        match document.resolve(object) {
            // The string form is a simple specification and can say nothing else: no file
            // system, no description, no embedded file.
            Object::String(bytes) => Some(Self {
                bytes: Some(bytes.to_vec()),
                ..Self::default()
            }),
            Object::Dictionary(dict) => Some(Self::from_dictionary(document, &dict)),
            _ => None,
        }
    }

    /// Reads Table 43.
    #[must_use]
    pub fn from_dictionary(document: &Document, dict: &Dictionary) -> Self {
        let string = |key: &str| match document.get_key(dict, key) {
            Object::String(bytes) => Some(bytes.to_vec()),
            _ => None,
        };
        let text = |key: &str| {
            string(key).and_then(|bytes| {
                let text = pdf_syntax::text_string(&bytes);
                (!text.is_empty()).then_some(text)
            })
        };

        // Table 43 puts `/F` first and the three platform keys after it; a specification with
        // none of the four says nothing about where its file is, which is a file the reader
        // could not name even if it had a filesystem.
        let (bytes, platform_specific) = match string("F") {
            Some(bytes) => (Some(bytes), false),
            None => (["DOS", "Mac", "Unix"].into_iter().find_map(&string), true),
        };

        Self {
            system: match document.get_key(dict, "FS").as_name() {
                None => FileSystem::Standard,
                Some(name) if name.as_bytes() == b"URL" => FileSystem::Url,
                Some(name) => {
                    FileSystem::Other(String::from_utf8_lossy(name.as_bytes()).into_owned())
                }
            },
            unicode_name: text("UF"),
            platform_specific: platform_specific && bytes.is_some(),
            bytes,
            description: text("Desc"),
            id: match document.get_key(dict, "ID").as_array() {
                Some(items) => match (items.first(), items.get(1)) {
                    (Some(Object::String(first)), Some(Object::String(second))) => {
                        Some([first.to_vec(), second.to_vec()])
                    }
                    _ => None,
                },
                None => None,
            },
            volatile: matches!(document.get_key(dict, "V"), Object::Boolean(true)),
            embedded: document.get_key(dict, "EF").as_dict().is_some(),
            payload: EncryptedPayload::read(document, dict),
        }
    }

    /// The name to show a person, and the one place a `/F` byte string becomes text.
    ///
    /// ISO 32000-2 §7.11.3's Table 43 states the precedence twice, once on each entry:
    ///
    /// > A PDF reader shall use the value of the UF key, when present, instead of the F key.
    ///
    /// `/UF` first, exactly as Table 43 requires; a `/F` is decoded by §7.9.2.2 only because a
    /// name has to be shown *somehow*, and `PDFDocEncoding` is the encoding the standard names
    /// for a byte string it displays. That decoding is a display decision and is not applied to
    /// [`FileSpec::components`], which stay bytes.
    #[must_use]
    pub fn display_name(&self) -> Option<String> {
        if let Some(name) = &self.unicode_name {
            return Some(name.clone());
        }
        self.bytes
            .as_ref()
            .map(|bytes| pdf_syntax::text_string(bytes))
            .filter(|name| !name.is_empty())
    }

    /// The URL this specification names, if `/FS` is `/URL` (§7.11.5).
    ///
    /// > When the FS entry in a file specification dictionary has the value URL , the value of
    /// > the F entry in that dictionary is not a file specification string, but a uniform
    /// > resource locator (URL) of the form defined in Internet RFC 3986 .
    ///
    /// `/F` rather than `/UF`, because the clause names `/F`: a URL is not a path and Table 43's
    /// `/UF` rule is about the two spellings of one path.
    #[must_use]
    pub fn url(&self) -> Option<String> {
        if self.system != FileSystem::Url {
            return None;
        }
        self.bytes
            .as_ref()
            .map(|bytes| pdf_syntax::text_string(bytes))
    }

    /// §7.11.2.1's components, as bytes, with the escapes removed.
    ///
    /// > The standard format for representing a simple file specification in string form divides
    /// > the string into component substrings separated by the SOLIDUS character (2Fh) (/).
    ///
    /// > If a component contains one or more literal SOLIDIUS character, each shall be preceded
    /// > by a REVERSE SOLIDUS (5Ch) (\\), which in turn shall be preceded by another REVERSE
    /// > SOLIDUS to indicate that it is part of the string and not an escape character.
    ///
    /// So the escape a *component* carries is two bytes in the string — the outer REVERSE
    /// SOLIDUS is the PDF string syntax's and is gone by the time this sees it, which is why
    /// this looks for `\/` and not `\\/`. The clause's own example, `(in\\/out)`, is a
    /// four-byte string `in\/out` after §7.3.4.2 and one component, `in/out`, after this.
    ///
    /// Empty for a URL specification, which is not a path (§7.11.5), and for a specification
    /// with no `/F` at all. An *empty component* is not the same thing and is kept: "[a]ny of
    /// the components may be empty", and an absolute specification's leading SOLIDUS produces
    /// one.
    #[must_use]
    pub fn components(&self) -> Vec<Vec<u8>> {
        if self.system == FileSystem::Url {
            return Vec::new();
        }
        let Some(bytes) = &self.bytes else {
            return Vec::new();
        };

        let mut components: Vec<Vec<u8>> = vec![Vec::new()];
        let mut escaped = false;
        for &byte in bytes {
            if escaped {
                // Only a SOLIDUS is escapable here; anything else means the file wrote a
                // REVERSE SOLIDUS the clause does not explain, and dropping it would change
                // the name. Both bytes are kept.
                if byte != b'/'
                    && let Some(last) = components.last_mut()
                {
                    last.push(b'\\');
                }
                if let Some(last) = components.last_mut() {
                    last.push(byte);
                }
                escaped = false;
                continue;
            }
            match byte {
                b'\\' => escaped = true,
                b'/' if components.len() < MAX_COMPONENTS => components.push(Vec::new()),
                other => {
                    if let Some(last) = components.last_mut() {
                        last.push(other);
                    }
                }
            }
        }
        // A trailing REVERSE SOLIDUS escapes nothing; it is a byte of the name.
        if escaped && let Some(last) = components.last_mut() {
            last.push(b'\\');
        }
        components
    }

    /// §7.11.2.2: whether this specification begins at the root of its file system.
    ///
    /// > A simple file specification that begins with a SOLIDUS shall be an absolute file
    /// > specification.
    ///
    /// A URL specification is neither: RFC 3986 decides what its path means, and §7.11.5 hands
    /// it to that document rather than to this rule.
    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.system != FileSystem::Url
            && self
                .bytes
                .as_ref()
                .is_some_and(|bytes| bytes.first() == Some(&b'/'))
    }

    /// §7.11.2.2's resolution of a relative specification against the file that contains it.
    ///
    /// > In the case of other file systems, a relative file specification shall be converted to
    /// > an absolute file specification by removing the file name component from the
    /// > specification of the containing PDF file and appending the relative file specification
    /// > in its place.
    ///
    /// > The special component . . (two PERIODs) (2Eh) can be used in a relative file
    /// > specification to move up a level in the file system hierarchy. After an absolute
    /// > specification has been derived, when the component immediately preceding . . is not
    /// > another . ., the two cancel each other; both are eliminated from the file specification
    /// > and the process is repeated.
    ///
    /// Returns the *named* components, still as bytes: the empty component an absolute
    /// specification's leading SOLIDUS produces is a root marker rather than a level, so it is
    /// not among them and [`FileSpec::is_absolute`] is what carries it.
    ///
    /// **Nothing in this program calls this to
    /// open anything** — it exists so that a refusal can name the file the document meant rather
    /// than the string it wrote, and so that the rule is stated where the clause can be checked
    /// against it.
    ///
    /// `None` for a URL specification, whose resolution is RFC 3986's and is `crate::uri`'s: the
    /// clause defers there by name, and §7.11.2.2 adds a restriction on top which
    /// [`FileSpec::is_valid_relative_url`] states.
    #[must_use]
    pub fn resolve_against(&self, containing: &[Vec<u8>]) -> Option<Vec<Vec<u8>>> {
        if self.system == FileSystem::Url {
            return None;
        }
        let mut resolved = if self.is_absolute() {
            Vec::new()
        } else {
            // "removing the file name component from the specification of the containing PDF
            // file": the last component is the file's own name, whatever it is.
            let keep = containing.len().saturating_sub(1);
            containing
                .get(..keep)
                .unwrap_or_default()
                .iter()
                // The containing specification's own leading empty component is its root
                // marker, and a doubled separator makes another; neither names a level, so
                // neither survives into the result for the same reason `..` cancels one.
                .filter(|component| !component.is_empty())
                .cloned()
                .collect()
        };

        for component in self.components() {
            match component.as_slice() {
                // A leading empty component is the absolute specification's own SOLIDUS, and
                // an empty component elsewhere is a doubled separator; neither names a level.
                b"" => {}
                b".." => {
                    // "when the component immediately preceding .. is not another .., the two
                    // cancel each other" — and where there is nothing to cancel, the `..` is
                    // kept, because dropping it would silently move the target up to the root.
                    match resolved.last().map(Vec::as_slice) {
                        Some(b"..") | None => resolved.push(component),
                        Some(_) => {
                            resolved.pop();
                        }
                    }
                }
                _ => resolved.push(component),
            }
        }
        Some(resolved)
    }

    /// §7.11.2.2's restriction on a relative specification under a URL file system.
    ///
    /// > In addition, such URL-based relative file specifications shall be limited to paths as
    /// > defined in Internet RFC 3986 . The scheme, network location/login, fragment identifier,
    /// > query information, and parameter sections shall not be allowed.
    ///
    /// **A security rule wearing a syntax rule's clothes**, and the reason it is checked rather
    /// than assumed: a relative specification that smuggles in an authority resolves against a
    /// *different host* from the document's. Nothing here fetches a URL, so what this defends is
    /// the day something does.
    #[must_use]
    pub fn is_valid_relative_url(url: &str) -> bool {
        // A scheme is what RFC 3986 section 3.1 defines, and its own section 4.2 rule for
        // telling a relative reference from an absolute one is whether a colon appears before
        // any slash, question mark or hash.
        let has_scheme = url.find(':').is_some_and(|colon| {
            url[..colon]
                .chars()
                .all(|c| c != '/' && c != '?' && c != '#')
        });
        !has_scheme
            && !url.starts_with("//")
            && !url.contains('?')
            && !url.contains('#')
            && !url.contains(';')
    }
}

#[cfg(test)]
mod tests {
    use super::{EncryptedPayload, FileSpec, FileSystem};
    use pdf_syntax::{Dictionary, Document, Object};

    /// Builds a document from object bodies numbered from 1, the catalog first.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// Object 3 of [`wrapper`], which is the file specification.
    fn specification(doc: &Document) -> Dictionary {
        let object = doc.resolve(&Object::Reference(pdf_syntax::ObjectId::new(3, 0)));
        object.as_dict().cloned().expect("the file specification")
    }

    /// The specification §7.6.7's EXAMPLE writes, with `extra` replacing its `/EP`.
    fn wrapper(extra: &str) -> Document {
        document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            &format!(
                "<< /Type /Filespec /F (AcmeCustomCrypto Protected PDF.pdf) \
                 /UF (AcmeCustomCrypto Protected PDF.pdf) /EF << /F 4 0 R >> \
                 /AFRelationship /EncryptedPayload {extra} >>"
            ),
            "<< /Type /EmbeddedFile /Length 3 >>\nstream\nhi\nendstream",
        ])
    }

    /// The specification in §7.6.7's own EXAMPLE, read — and the same one without its `/EP`.
    ///
    /// ISO 32000-2 §7.6.7:
    ///
    /// > The file specification dictionary for the encrypted payload shall include the
    /// > AFRelationship key with a value of EncryptedPayload , and shall include an encrypted
    /// > payload dictionary (see "Table 28 -Entries in an encrypted payload dictionary") with
    /// > details of the cryptographic filter needed to decrypt the encrypted payload.
    ///
    /// The calibration is the second half (trap 13): the same dictionary with the entry removed
    /// answers `None`, so the reader is measuring the entry rather than the fixture. No document
    /// in any corpus on this disk states one, which is why this is a fixture at all.
    #[test]
    fn an_encrypted_payload_is_named_by_its_cryptographic_filter() {
        let doc =
            wrapper("/EP << /Type /EncryptedPayload /Subtype /AcmeCustomCrypto /Version /1.0 >>");
        let spec = FileSpec::from_dictionary(&doc, &specification(&doc));
        let payload = spec.payload.as_ref().expect("Table 43's /EP");
        assert_eq!(payload.filter.as_deref(), Some("AcmeCustomCrypto"));
        assert_eq!(
            payload.version.as_deref(),
            Some("1.0"),
            "a name, kept as the producer spelled it rather than parsed as a number"
        );
        assert!(payload.permitted_type);

        let plain = wrapper("");
        assert!(
            FileSpec::from_dictionary(&plain, &specification(&plain))
                .payload
                .is_none(),
            "no /EP is not an encrypted payload, whatever the /AFRelationship says"
        );
    }

    /// Table 28's two producer-side rules, recorded rather than refused.
    ///
    /// `/Subtype` is the table's one Required entry and `/Type`, "if present, shall be
    /// EncryptedPayload". A file breaking either has still said it carries a payload, which is
    /// the fact a reader with no handler for it acts on.
    #[expect(
        clippy::doc_markdown,
        reason = "the doc quotes Table 28's cell verbatim, and a quotation is not marked up"
    )]
    #[test]
    fn a_payload_missing_its_required_subtype_is_still_a_payload() {
        let doc = wrapper("/EP << /Type /Filespec >>");
        let payload = FileSpec::from_dictionary(&doc, &specification(&doc))
            .payload
            .expect("the dictionary is present");
        assert_eq!(
            payload,
            EncryptedPayload {
                filter: None,
                version: None,
                permitted_type: false,
            }
        );
    }

    /// A specification built straight from a string, as §7.11.1's simple form.
    fn spec(path: &str) -> FileSpec {
        FileSpec {
            bytes: Some(path.as_bytes().to_vec()),
            ..FileSpec::default()
        }
    }

    /// ISO 32000-2 §7.11.2.1's own example, which is the whole of the escape rule.
    ///
    /// > EXAMPLE
    /// >
    /// > (in\\/out)
    /// >
    /// > represents the file name
    /// >
    /// > in/out
    ///
    /// The outer REVERSE SOLIDUS belongs to §7.3.4.2's string syntax and is gone before this
    /// sees the bytes, so what arrives is `in\/out` and what comes out is one component.
    #[test]
    fn a_escaped_solidus_is_part_of_a_component_rather_than_a_separator() {
        assert_eq!(spec("in\\/out").components(), vec![b"in/out".to_vec()]);
        // Without the escape the same bytes are two components, which is the contrast that
        // makes the rule mean anything.
        assert_eq!(
            spec("in/out").components(),
            vec![b"in".to_vec(), b"out".to_vec()]
        );
    }

    /// "Any of the components may be empty", including the one a leading SOLIDUS makes.
    #[test]
    fn an_empty_component_is_a_component() {
        assert_eq!(
            spec("/a//b").components(),
            vec![b"".to_vec(), b"a".to_vec(), b"".to_vec(), b"b".to_vec()]
        );
    }

    /// ISO 32000-2 §7.11.2.2's EXAMPLE 1, verbatim.
    ///
    /// > The relative file specification ArtFiles/Figure1.pdf appearing in a PDF file whose
    /// > specification is /HardDisk/PDFDocuments/AnnualReport/Summary.pdf yields the absolute
    /// > specification
    #[test]
    fn a_relative_specification_replaces_the_containing_files_name() {
        let containing = spec("/HardDisk/PDFDocuments/AnnualReport/Summary.pdf").components();
        let resolved = spec("ArtFiles/Figure1.pdf")
            .resolve_against(&containing)
            .expect("a path rather than a URL");
        assert_eq!(
            resolved,
            vec![
                b"HardDisk".to_vec(),
                b"PDFDocuments".to_vec(),
                b"AnnualReport".to_vec(),
                b"ArtFiles".to_vec(),
                b"Figure1.pdf".to_vec()
            ]
        );
    }

    /// ISO 32000-2 §7.11.2.2's EXAMPLE 2: the `..` components cancel what precedes them.
    ///
    /// > The relative file specification from Example 1 in this subclause using the .. (two
    /// > PERIODs) special component
    ///
    /// `../../ArtFiles/Figure1.pdf` against the same containing file yields
    /// `/HardDisk/ArtFiles/Figure1.pdf` — the clause states the answer, which is what makes it
    /// a test rather than a reading.
    #[test]
    fn two_periods_cancel_the_component_before_them() {
        let containing = spec("/HardDisk/PDFDocuments/AnnualReport/Summary.pdf").components();
        let resolved = spec("../../ArtFiles/Figure1.pdf")
            .resolve_against(&containing)
            .expect("a path rather than a URL");
        assert_eq!(
            resolved,
            vec![
                b"HardDisk".to_vec(),
                b"ArtFiles".to_vec(),
                b"Figure1.pdf".to_vec()
            ]
        );
    }

    /// A `..` with nothing before it is kept rather than dropped.
    ///
    /// The clause cancels a `..` only "when the component immediately preceding .. is not
    /// another . ." — it says nothing about one with no predecessor at all, and the two
    /// available answers differ: dropping it silently moves the target to the root, which is a
    /// path the document did not write. Keeping it leaves a specification this program declines
    /// to open anyway, and says so.
    #[test]
    fn a_two_period_component_with_nothing_to_cancel_is_kept() {
        assert_eq!(
            spec("../x").resolve_against(&[]),
            Some(vec![b"..".to_vec(), b"x".to_vec()])
        );
    }

    /// An absolute specification ignores the file that contains it.
    #[test]
    fn an_absolute_specification_starts_at_the_root() {
        let containing = spec("/a/b/c.pdf").components();
        assert!(spec("/x/y.pdf").is_absolute());
        assert!(!spec("x/y.pdf").is_absolute());
        assert_eq!(
            spec("/x/y.pdf").resolve_against(&containing),
            Some(vec![b"x".to_vec(), b"y.pdf".to_vec()])
        );
    }

    /// §7.11.5's URL form is not a path, and none of the path rules reach it.
    #[test]
    fn a_url_specification_is_not_split_into_components() {
        let url = FileSpec {
            system: FileSystem::Url,
            bytes: Some(b"ftp://www.beatles.com/Movies/AbbeyRoad.mov".to_vec()),
            ..FileSpec::default()
        };
        assert_eq!(
            url.url().as_deref(),
            Some("ftp://www.beatles.com/Movies/AbbeyRoad.mov")
        );
        assert!(url.components().is_empty());
        assert!(!url.is_absolute());
        assert_eq!(url.resolve_against(&[]), None);
    }

    /// §7.11.2.2's restriction on a relative URL specification, which is a security rule.
    #[test]
    fn a_relative_url_specification_may_be_a_path_and_nothing_else() {
        assert!(FileSpec::is_valid_relative_url("images/figure1.png"));
        assert!(FileSpec::is_valid_relative_url("../figure1.png"));
        // Each of these is one of the five sections the clause forbids.
        assert!(!FileSpec::is_valid_relative_url("http://elsewhere/x"));
        assert!(!FileSpec::is_valid_relative_url("//elsewhere/x"));
        assert!(!FileSpec::is_valid_relative_url("x#fragment"));
        assert!(!FileSpec::is_valid_relative_url("x?query=1"));
        assert!(!FileSpec::is_valid_relative_url("x;parameter"));
    }

    /// `/UF` outranks `/F`, and a `/F` is decoded only to be shown.
    #[test]
    fn the_unicode_name_is_used_instead_of_the_byte_string() {
        let both = FileSpec {
            unicode_name: Some("réponse.pdf".to_owned()),
            bytes: Some(b"reponse.pdf".to_vec()),
            ..FileSpec::default()
        };
        assert_eq!(both.display_name().as_deref(), Some("réponse.pdf"));
        assert_eq!(
            both.components(),
            vec![b"reponse.pdf".to_vec()],
            "and the components stay the bytes /F wrote, whatever /UF spells"
        );
    }
}
