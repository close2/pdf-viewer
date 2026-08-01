//! ISO 32000-2 §12.7.8's Forms Data Format: a second file that says what this form holds.
//!
//! §12.7.8.1 states what FDF is for, and the last of its three uses is the one that reaches a
//! screen:
//!
//! > FDF can be used when submitting form data to a server, receiving the response, and
//! > incorporating it into the interactive form. It can also be used to export form data to
//! > stand-alone files that can be stored, transmitted electronically, and imported back into
//! > the corresponding PDF interactive form.
//!
//! *Incorporating it into the interactive form* is a display change and nothing else: §12.7.4.3
//! lays a field's value out from its `/DA` string, and importing replaces the value it lays out.
//! That is the whole reason this module exists in a renderer — the same argument §12.7.6.3's
//! reset-form action was implemented under (ADR 0087), one step further along. A reset makes a
//! field's value its own `/DV`; an import makes it a value from *another file*.
//!
//! # An FDF file is read by the machinery already here
//!
//! §12.7.8.1: "FDF is based on PDF; it uses the same syntax and has essentially the same file
//! structure". So [`pdf_syntax::Document`] opens one — its lexer, its parser, its object model
//! and its recovery-by-scanning all apply unchanged — and the four differences the clause lists
//! are all *relaxations*: no cross-reference table is required, there are no incremental
//! updates, the body is one required object, and a stream length is direct. A reader that
//! survives a damaged PDF survives all four.
//!
//! Two things are genuinely FDF's own and are handled here. The header is `%FDF-1.n` rather
//! than `%PDF-n.m` (§12.7.8.2.2), which `pdf_syntax::xref` now recognises so that §7.5.2's
//! "byte offsets shall be calculated from the PERCENT SIGN" measures from the right sign; and
//! the object hierarchy's root is Table 245's `/FDF` dictionary rather than §7.7.2's catalog.
//!
//! # What importing *is*, in one sentence the clause writes down
//!
//! §12.7.8.3.2:
//!
//! > Unless otherwise indicated in the table, importing a field causes the values of the entries
//! > in the FDF field dictionary to replace those of the corresponding entries in the field with
//! > the same fully qualified name in the target document.
//!
//! Two halves, and both are load-bearing. *Replace* is why [`Import`] is a set of overrides
//! rather than a patch applied to the file: nothing here writes to the target document, exactly
//! as nothing writes a reset. *The same fully qualified name* is why [`FdfField::name`] is built
//! by concatenating `/T` down the `/Kids` chain with §12.7.4.2's full stop — an FDF file states
//! its fields as a tree and a target document matches them by the flattened name.
//!
//! The words "unless otherwise indicated" are the flag entries, and Table 249 indicates
//! precisely: `/Ff` replaces, `/SetFf` and `/ClrFf` modify, and "[t]his entry shall be ignored
//! if an `Ff` entry is present". [`FlagChange`] is that arithmetic, once, for both the field
//! flags and the widget's annotation flags.
//!
//! # What is read and not applied, and why each
//!
//! Every one of these is *named* on [`FormsData::owed`] rather than skipped, which is principle
//! 3's requirement:
//!
//! - **`/Pages`** (Table 246) adds template pages to the target document, which needs §12.7.7's
//!   named pages and a page tree this reader may extend. Read as a count and refused.
//! - **`/Annots`** (Table 254) carries annotations belonging to no document, each naming the
//!   page it attaches to. Drawing one means resolving its `/AP` against the *FDF* file's objects
//!   while placing it on the target's page, which is a second document reaching into the
//!   interpreter — a real design question rather than an oversight. [`FdfAnnotation`] is what
//!   the file says; nothing draws it yet.
//! - **`/JavaScript`** (Table 248) is on `CLAUDE.md`'s closed exclusion list.
//! - **`/EmbeddedFDFs`** is FDF files inside this one, which §7.11.4 reads as attachments and
//!   which would need this module to recurse into an encryption scheme (Table 247) that ISO
//!   32000-2 deprecates in the same paragraph that defines it.
//! - **`/Differences`** is the target document's own incremental updates, carried for a server;
//!   applying it would mean *writing* the target file, which principle 5 puts outside this
//!   project.
//! - **`/RV`**, **`/AP`**, **`/APRef`** and **`/IF`** on a field: XFA rich text (excluded), a
//!   push-button's appearance streams living in the FDF file, appearances in *other* PDF files,
//!   and the icon fit dictionary that would place them.
//!
//! # No corpus document exercises any of this
//!
//! Not one of the 974 pdf.js documents carries an import-data action and none is accompanied by
//! an FDF file, which trap 8 says is the ordinary case for a clause rather than a reason to
//! skip it: a corpus finds what documents contain, not what the specification says. The tests
//! below are therefore synthetic and state one rule apiece.

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Most fields flattened out of one FDF file's `/Fields` tree.
///
/// A form a person fills in has tens of fields and a generated one has thousands; a file
/// claiming more than this is making a reader work rather than describing a form.
const MAX_FIELDS: usize = 65536;

/// How deep the `/Kids` chain is followed.
///
/// §12.7.4.2's fully qualified name is built by concatenation, so a chain this long describes a
/// name no interface could show. The bound is also what makes a `/Kids` cycle terminate — Table
/// 249 permits a *direct* child, so a cycle needs an indirect one, and a file may write one.
const MAX_FIELD_DEPTH: usize = 64;

/// Most annotations listed from one FDF file's `/Annots`.
const MAX_ANNOTATIONS: usize = 4096;

/// Why an FDF file could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormsDataError {
    /// The trailer's `/Root` names no dictionary (§12.7.8.2.4, Table 244).
    #[error("the FDF trailer's /Root does not resolve to a dictionary")]
    NoCatalog,
    /// The catalog states no `/FDF` dictionary, which Table 245 makes its one required entry.
    #[error("the FDF catalog has no /FDF dictionary, which Table 245 requires")]
    NotFormsData,
}

/// One FDF file, read.
#[derive(Debug, Clone, PartialEq)]
pub struct FormsData {
    /// Table 245's `/Version`, "[t]he version of the FDF specification to which this FDF file
    /// conforms … if later than the version specified in the file's header".
    ///
    /// A name in the file rather than a number, which the table says in as many words: "[t]he
    /// value of this entry is a name object, not a number".
    pub version: Option<String>,
    /// Table 246's `/F`: "[t]he source file or target file", as the file spells it.
    ///
    /// A name for a person, not a path this program will open — the same position `/UF` and
    /// `/F` take in [`crate::attachment`]. A caller deciding whether this FDF belongs with the
    /// document it has open compares it, and [`Self::identifier`] is the stronger comparison.
    pub source: Option<String>,
    /// Table 246's `/ID`, "an array of two byte strings constituting a file identifier … taken
    /// from the ID entry in the file's trailer dictionary".
    ///
    /// §14.4 makes the first element permanent and the second a version, so a caller checking
    /// that an FDF belongs to a document compares the first and learns from the second whether
    /// the document has been revised since the data was exported.
    pub identifier: Option<[Vec<u8>; 2]>,
    /// Table 246's `/Fields`, flattened to §12.7.4.2's fully qualified names.
    pub fields: Vec<FdfField>,
    /// Table 246's `/Status`: "[a] status string that shall be displayed indicating the result
    /// of an action, typically a submit-form action".
    ///
    /// *Shall be displayed* — so this is not diagnostics, it is a message the server sent to the
    /// person at the screen, and a caller that drops it has lost the file's whole answer.
    pub status: Option<String>,
    /// Table 246's `/Encoding`, which decides how a value's bytes become characters.
    pub encoding: Encoding,
    /// Table 254's annotations, read and drawn by nothing.
    pub annotations: Vec<FdfAnnotation>,
    /// How many Table 251 page dictionaries `/Pages` states, which is how many template pages
    /// this file would add to the target document.
    pub pages: usize,
    /// Table 246's `/Target`, "[t]he name of a browser frame in which the underlying PDF
    /// document shall be opened".
    pub target: Option<String>,
    /// What this file states and this program does not act on, each by name.
    ///
    /// Not errors and not silences: a file may carry all of them and still import its values
    /// correctly. The module comment argues each one.
    pub owed: Vec<&'static str>,
}

/// Table 246's `/Encoding`: how an FDF field's value, option or name is encoded.
///
/// The entry's condition is as important as its value. It applies to a string "that does not
/// begin with the Unicode prefix ZERO WIDTH NO-BREAK SPACE (U+FEFF)" — which is §7.9.2.2.1's
/// UTF-16BE marker, so a string carrying one is decoded as a text string whatever this says.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Encoding {
    /// The table's default, and §7.9.2.2's own answer for a string with no marker.
    #[default]
    PdfDocEncoding,
    /// `utf_8` or `utf_16`, which §7.9.2.2 already decodes from the string's own prefix.
    Unicode,
    /// One of the table's four registered character sets — `Shift_JIS`, `BigFive`, `GBK`,
    /// `UHC` — named and not decoded.
    ///
    /// Each is a character set standard published elsewhere, and carrying its table here is the
    /// same decision Table 116's predefined `CMap`s are refused under: guessing produces
    /// plausible text that says something else. A value in one of these is reported rather than
    /// mojibake.
    Registered(String),
}

impl Encoding {
    /// Table 246's four named values, plus the two the clause says are Unicode.
    fn read(name: Option<&[u8]>) -> Self {
        match name {
            None | Some(b"PDFDocEncoding") => Self::PdfDocEncoding,
            Some(b"utf_8" | b"utf_16") => Self::Unicode,
            Some(other) => Self::Registered(String::from_utf8_lossy(other).into_owned()),
        }
    }

    /// Decodes one of this file's strings, or says which character set it would take.
    ///
    /// The marker test is the clause's: a string beginning U+FEFF is a text string by
    /// §7.9.2.2.1 and this entry does not reach it. Everything else takes `/Encoding`, whose
    /// default and whose two Unicode values are all [`pdf_syntax::text_string`] — which chooses
    /// among exactly those three by the same prefix rule.
    fn decode(&self, bytes: &[u8]) -> Result<String, &str> {
        const UTF16BE_MARKER: [u8; 2] = [254, 255];
        if let Self::Registered(charset) = self
            && !bytes.starts_with(&UTF16BE_MARKER)
        {
            return Err(charset);
        }
        Ok(pdf_syntax::text_string(bytes))
    }
}

/// One field of an FDF file. Table 249.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfField {
    /// §12.7.4.2's fully qualified name, built from every `/T` on the path to this field.
    ///
    /// Table 249 makes `/T` required, and a field without one has no name to match against the
    /// target document — so such a node contributes its `/Kids` and nothing of its own.
    pub name: String,
    /// Table 249's `/V`, "[t]he field's value, whose format varies depending on the field type".
    ///
    /// Held as the object the file states rather than a string, because the format is the field
    /// type's: a text field's value is a string, a check box's is a name, a list box's is an
    /// array. §12.7.4.3 already knows how to lay out each of them, which is the point.
    pub value: Option<Object>,
    /// `/Ff`, `/SetFf` and `/ClrFf` as the one arithmetic they describe.
    pub flags: FlagChange,
    /// `/F`, `/SetF` and `/ClrF`: the same arithmetic over §12.5.3's annotation flags.
    pub annotation_flags: FlagChange,
    /// Table 249's `/Opt`, "[r]equired; choice fields only", as the strings it presents.
    ///
    /// Each element is "[a] text string representing one of the available options" or "[a]
    /// two-element array consisting of a text string … and a default appearance string"; the
    /// first element of the pair is the option either way, which is what this holds.
    pub options: Option<Vec<String>>,
    /// What this field states and this program does not apply, by entry name.
    pub owed: Vec<&'static str>,
}

/// Table 249's three-entry pattern for a flag word, which appears twice in that table.
///
/// The table states the precedence itself, for both triples: `/SetFf` and `/ClrFf` "shall be
/// ignored if an `Ff` entry is present", and `/SetF` and `/ClrF` likewise. So the three entries
/// are one decision with three shapes rather than three independent overrides — which is why
/// this is an enum and why [`Self::applied_to`] is the only place the arithmetic is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlagChange {
    /// None of the three entries is present: the target document's own flags stand.
    #[default]
    Unchanged,
    /// `/Ff` or `/F`, which "shall replace that of the … entry in the form's corresponding"
    /// dictionary.
    Replace(i64),
    /// `/SetFf` then `/ClrFf`, in that order — "[i]f a `SetFf` entry is also present … it shall
    /// be applied before this entry".
    Modify {
        /// Bits to turn on: "[b]its equal to 1 in `SetFf` shall cause the corresponding bits in
        /// `Ff` to be set to 1".
        set: i64,
        /// Bits to turn off, applied second.
        clear: i64,
    },
}

impl FlagChange {
    /// Table 249's `/Ff`, `/SetFf` and `/ClrFf` for one field, with the table's own precedence.
    fn read(
        document: &Document,
        field: &Dictionary,
        replace: &str,
        set: &str,
        clear: &str,
    ) -> Self {
        if let Some(flags) = document.get_key(field, replace).as_integer() {
            return Self::Replace(flags);
        }
        let set_bits = document.get_key(field, set).as_integer();
        let clear_bits = document.get_key(field, clear).as_integer();
        match (set_bits, clear_bits) {
            (None, None) => Self::Unchanged,
            _ => Self::Modify {
                set: set_bits.unwrap_or_default(),
                clear: clear_bits.unwrap_or_default(),
            },
        }
    }

    /// This change applied to the flag word a target document states.
    #[must_use]
    pub fn applied_to(self, existing: i64) -> i64 {
        match self {
            Self::Unchanged => existing,
            Self::Replace(flags) => flags,
            Self::Modify { set, clear } => (existing | set) & !clear,
        }
    }

    /// Whether this change leaves every flag word it is applied to alone.
    #[must_use]
    pub fn is_unchanged(self) -> bool {
        self == Self::Unchanged
    }
}

/// One annotation carried by an FDF file. Table 254 over §12.5.6's own dictionaries.
///
/// §12.7.8.1: FDF "can be used to define a container for annotations that are separate from the
/// PDF document to which they apply", which is why a page number is part of the annotation
/// rather than of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfAnnotation {
    /// Table 254's `/Page`: "the page of the source document to which the annotation is
    /// attached", zero-based as every page index in this tree is.
    pub page: Option<usize>,
    /// §12.5.2's `/Subtype`, as the file spells it.
    ///
    /// Table 246 excludes six of Table 171's types from an FDF file — `Link`, `Movie`,
    /// `Widget`, `PrinterMark`, `Screen` and `TrapNet` — and this keeps whatever the file wrote,
    /// because a reader that silently drops an excluded subtype has hidden a malformed file.
    pub subtype: Option<String>,
    /// The dictionary itself, whose references resolve in the *FDF* file and nowhere else.
    pub dictionary: Dictionary,
}

impl FormsData {
    /// Reads an FDF file's catalog and everything under it.
    ///
    /// `document` is an FDF file opened by [`pdf_syntax::Document`], which §12.7.8.1 makes the
    /// right reader for one.
    ///
    /// # Errors
    ///
    /// [`FormsDataError::NoCatalog`] where the trailer's `/Root` resolves to no dictionary, and
    /// [`FormsDataError::NotFormsData`] where that dictionary has no `/FDF` — which is how a PDF
    /// handed to this function identifies itself as one.
    pub fn read(document: &Document) -> Result<Self, FormsDataError> {
        let catalog = document.catalog().map_err(|_| FormsDataError::NoCatalog)?;
        let Some(fdf) = document.get_key(&catalog, "FDF").as_dict().cloned() else {
            return Err(FormsDataError::NotFormsData);
        };

        let encoding = Encoding::read(
            document
                .get_key(&fdf, "Encoding")
                .as_name()
                .map(|name| name.as_bytes().to_vec())
                .as_deref(),
        );

        let mut owed = Vec::new();
        let mut fields = Vec::new();
        read_fields(
            document,
            &document.get_key(&fdf, "Fields"),
            "",
            0,
            &encoding,
            &mut fields,
        );

        let pages = document
            .get_key(&fdf, "Pages")
            .as_array()
            .map_or(0, <[Object]>::len);
        if pages > 0 {
            owed.push("/Pages: template pages, which need §12.7.7's named pages");
        }
        // Table 246 states two exclusions between these three entries, and both are rules about
        // a *writer*: "[t]his entry and the Pages entry shall not both be present" of `/Fields`
        // and of `/Status`. A reader meeting a file that breaks them has data it can use and no
        // clause telling it which to prefer, so both are read and the contradiction is named.
        if pages > 0 && !fields.is_empty() {
            owed.push("/Fields and /Pages are both present, which Table 246 forbids");
        }

        let annotations = read_annotations(document, &document.get_key(&fdf, "Annots"));
        for (key, why) in [
            (
                "JavaScript",
                "/JavaScript: document-level scripts, excluded",
            ),
            ("EmbeddedFDFs", "/EmbeddedFDFs: FDF files inside this one"),
            (
                "Differences",
                "/Differences: the target document's own incremental updates",
            ),
        ] {
            if !document.get_key(&fdf, key).is_null() {
                owed.push(why);
            }
        }
        if !annotations.is_empty() {
            owed.push("/Annots: annotations belonging to no document, read and not drawn");
        }

        Ok(Self {
            version: document
                .get_key(&catalog, "Version")
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
            source: document
                .get_key(&fdf, "F")
                .as_string()
                .map(pdf_syntax::text_string),
            identifier: identifier(document, &fdf),
            fields,
            status: document
                .get_key(&fdf, "Status")
                .as_string()
                .map(pdf_syntax::text_string),
            encoding,
            annotations,
            pages,
            target: document
                .get_key(&fdf, "Target")
                .as_string()
                .map(pdf_syntax::text_string),
            owed,
        })
    }

    /// Whether this file names the document it belongs to, and agrees with it.
    ///
    /// §14.4's file identifier is two byte strings, of which the first "shall be a permanent
    /// identifier based on the contents of the file at the time it was originally created" — so
    /// comparing the first elements answers "is this the same document", and the second answers
    /// "is it the same revision", which this deliberately does not ask: a form filled in against
    /// one revision is still that form's data.
    ///
    /// `None` where either file states no `/ID`, which is a question that cannot be asked rather
    /// than an answer of no.
    #[must_use]
    pub fn belongs_to(&self, target: &Document) -> Option<bool> {
        let mine = self.identifier.as_ref()?;
        let theirs = target.trailer().get("ID")?.as_array()?;
        let first = theirs.first()?.as_string()?;
        Some(mine[0] == first)
    }
}

/// Table 246's `/ID`, "an array of two byte strings".
///
/// `None` unless the array has both elements as strings: a one-element or malformed `/ID`
/// identifies nothing, and half an identifier compared against a whole one would answer.
fn identifier(document: &Document, fdf: &Dictionary) -> Option<[Vec<u8>; 2]> {
    let stated = document.get_key(fdf, "ID");
    let array = stated.as_array()?;
    let [first, second] = array else { return None };
    Some([
        document.resolve(first).as_string()?.to_vec(),
        document.resolve(second).as_string()?.to_vec(),
    ])
}

/// Walks one level of Table 249's `/Kids` tree, appending every named field it reaches.
///
/// `prefix` is §12.7.4.2's fully qualified name of the parent:
///
/// > For a field with no parent, the partial and fully qualified names are the same. For a field
/// > that is the child of another field, the fully qualified name shall be formed by appending
/// > the child field's partial name to the parent's fully qualified name, separated by a PERIOD
/// > (2Eh)
///
/// A node with no `/T` contributes no separator, because there is no partial name to separate —
/// which is the reading that makes a nameless intermediate node harmless rather than producing
/// `..` in the middle of a name no document could match.
fn read_fields(
    document: &Document,
    entry: &Object,
    prefix: &str,
    depth: usize,
    encoding: &Encoding,
    into: &mut Vec<FdfField>,
) {
    if depth >= MAX_FIELD_DEPTH || into.len() >= MAX_FIELDS {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(items) = resolved.as_array() else {
        return;
    };
    for item in items {
        if into.len() >= MAX_FIELDS {
            return;
        }
        let resolved = document.resolve(item);
        let Some(field) = resolved.as_dict() else {
            continue;
        };
        // §12.7.8.3.1 applies `/Encoding` to a "field name that is a string" exactly as it does
        // to a value, so a name in a character set this program has no table for cannot be
        // turned into the string a target document would be matched against. Such a field is
        // still listed, under the bytes read as though they were `PDFDocEncoding` — it will
        // match nothing, and saying *why* it matched nothing is what the entry on `owed` is for.
        let (partial, name_owed) = match document.get_key(field, "T").as_string() {
            None => (None, false),
            Some(bytes) => match encoding.decode(bytes) {
                Ok(text) => (Some(text).filter(|name| !name.is_empty()), false),
                Err(_) => (Some(pdf_syntax::text_string(bytes)), true),
            },
        };
        let name = match (&partial, prefix.is_empty()) {
            (None, _) => prefix.to_owned(),
            (Some(partial), true) => partial.clone(),
            (Some(partial), false) => format!("{prefix}.{partial}"),
        };
        if partial.is_some() {
            into.push(read_field(
                document,
                field,
                name.clone(),
                encoding,
                name_owed,
            ));
        }
        read_fields(
            document,
            &document.get_key(field, "Kids"),
            &name,
            depth.saturating_add(1),
            encoding,
            into,
        );
    }
}

/// Table 249's entries for one field, with the flag arithmetic and the refusals.
fn read_field(
    document: &Document,
    field: &Dictionary,
    name: String,
    encoding: &Encoding,
    name_owed: bool,
) -> FdfField {
    let mut owed = Vec::new();
    if name_owed {
        owed.push("/T in a character set this program has no table for");
    }
    let stated = document.get_key(field, "V");
    // The value is kept as the file's own object rather than as a decoded string, because
    // §12.7.4.3 already knows how to lay each of Table 226's value types out and takes a string
    // through the same §7.9.2.2 route `Encoding::decode` takes it through here. Only the one
    // case where those two answers *differ* is acted on: a registered character set, refused at
    // the value rather than at the file, because an FDF file may name `Shift_JIS` and carry a
    // check box whose value is a *name*, which no character set reaches.
    let value = match &stated {
        Object::String(bytes) if encoding.decode(bytes).is_err() => {
            owed.push("/V in a character set this program has no table for");
            None
        }
        Object::Null => None,
        other => Some(other.clone()),
    };
    for (key, why) in [
        (
            "AP",
            "/AP: a push-button's appearance streams, in this file",
        ),
        ("APRef", "/APRef: appearances in other PDF files"),
        ("IF", "/IF: an icon fit dictionary, which places those"),
        ("RV", "/RV: XFA rich text, excluded"),
        ("A", "/A: an action to perform when the widget is activated"),
        ("AA", "/AA: §12.6.3's trigger events"),
    ] {
        if !document.get_key(field, key).is_null() {
            owed.push(why);
        }
    }
    FdfField {
        name,
        value,
        flags: FlagChange::read(document, field, "Ff", "SetFf", "ClrFf"),
        annotation_flags: FlagChange::read(document, field, "F", "SetF", "ClrF"),
        options: options(document, field, encoding),
        owed,
    }
}

/// Table 249's `/Opt`, in both the forms the table gives an element.
fn options(document: &Document, field: &Dictionary, encoding: &Encoding) -> Option<Vec<String>> {
    let stated = document.get_key(field, "Opt");
    let items = stated.as_array()?;
    Some(
        items
            .iter()
            .map(|item| {
                let resolved = document.resolve(item);
                // "A two-element array consisting of a text string representing one of the
                // available options and a default appearance string" — the option is the first
                // element, and a `/DA` for an item this program does not present is not read.
                let text = match &resolved {
                    Object::Array(pair) => {
                        pair.first().and_then(Object::as_string).map(<[u8]>::to_vec)
                    }
                    other => other.as_string().map(<[u8]>::to_vec),
                };
                text.map_or_else(String::new, |bytes| {
                    encoding.decode(&bytes).unwrap_or_else(|_| String::new())
                })
            })
            .collect(),
    )
}

/// Table 254's annotations, each with the page it says it belongs to.
fn read_annotations(document: &Document, entry: &Object) -> Vec<FdfAnnotation> {
    let resolved = document.resolve(entry);
    let Some(items) = resolved.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .take(MAX_ANNOTATIONS)
        .filter_map(|item| {
            let resolved = document.resolve(item);
            let dictionary = resolved.as_dict()?.clone();
            Some(FdfAnnotation {
                page: document
                    .get_key(&dictionary, "Page")
                    .as_integer()
                    .and_then(|page| usize::try_from(page).ok()),
                subtype: document
                    .get_key(&dictionary, "Subtype")
                    .as_name()
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
                dictionary,
            })
        })
        .collect()
}

/// What one FDF field says about one widget of the target document.
///
/// The unit is a widget rather than a field for [`crate::view::ViewState`]'s reason: §12.7.4.1's
/// field tree ends in the annotations that show a field, one field may have several, and what
/// this program draws is annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// Table 249's `/V`, which replaces the target field's own.
    ///
    /// `None` where the FDF field states no value, which §12.7.8.3.2's "replace" makes a field
    /// whose value is removed — the same state §12.7.6.3's reset leaves a field with no `/DV`
    /// in, and drawn the same way.
    pub value: Option<Object>,
    /// `/F`, `/SetF` and `/ClrF` over §12.5.3's annotation flags.
    pub annotation_flags: FlagChange,
    /// `/Ff`, `/SetFf` and `/ClrFf` over Table 227's field flags.
    pub field_flags: FlagChange,
}

/// Pairs an FDF file's fields with a target document's widgets, by fully qualified name.
///
/// The names on both sides are §12.7.4.2's, which is the whole of §12.7.8.3.2's matching rule.
/// A field the target document does not have is *not* an error and not silently dropped: it is
/// returned as its own list, because a file naming fields a form has not got is either the wrong
/// FDF for this document or a form that has changed, and a caller should be able to say which.
#[must_use]
pub fn match_to_document(
    data: &FormsData,
    widgets: &std::collections::BTreeMap<String, Vec<ObjectId>>,
) -> (Vec<(ObjectId, Import)>, Vec<String>) {
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    for field in &data.fields {
        let Some(ids) = widgets.get(&field.name) else {
            unmatched.push(field.name.clone());
            continue;
        };
        for id in ids {
            matched.push((
                *id,
                Import {
                    value: field.value.clone(),
                    annotation_flags: field.annotation_flags,
                    field_flags: field.flags,
                },
            ));
        }
    }
    (matched, unmatched)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an FDF file from a body, with the header §12.7.8.2.2 states and the trailer
    /// §12.7.8.2.4 does — and deliberately without a cross-reference table, which §12.7.8.1
    /// makes optional and which is how most real FDF files are written.
    fn fdf(body: &str) -> Document {
        let bytes = format!("%FDF-1.2\n{body}\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n");
        Document::open(bytes.into_bytes()).expect("an FDF file is opened by the PDF reader")
    }

    /// §12.7.8.2.1: an FDF file "shall be structured in essentially the same way as a PDF file",
    /// and §12.7.8.1 lists the differences as relaxations — so the reader that opens a PDF opens
    /// this, with no cross-reference table at all.
    #[test]
    fn an_fdf_file_is_read_by_the_pdf_reader() {
        let document =
            fdf("1 0 obj\n<< /FDF << /Fields [ << /T (name) /V (Ada) >> ] >> >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.fields.len(), 1);
        assert_eq!(data.fields[0].name, "name");
        assert_eq!(
            data.fields[0].value.as_ref().and_then(Object::as_string),
            Some(b"Ada".as_slice())
        );
    }

    /// A PDF is not an FDF file, and Table 245's one required entry is what says so.
    #[test]
    fn a_file_with_no_fdf_dictionary_is_not_forms_data() {
        let document = fdf("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj");
        assert_eq!(
            FormsData::read(&document),
            Err(FormsDataError::NotFormsData)
        );
    }

    /// §12.7.4.2's fully qualified name over Table 249's `/Kids`: the partial names of every
    /// ancestor, concatenated and separated by full stops. A node with no `/T` of its own adds
    /// no separator and is not itself a field.
    #[test]
    fn a_kids_tree_flattens_to_fully_qualified_names() {
        let document = fdf("1 0 obj\n<< /FDF << /Fields [\n\
             << /T (address) /Kids [ << /T (city) /V (Lovelace) >> << /T (zip) /V (12345) >> ] >>\n\
             << /Kids [ << /T (loose) /V (x) >> ] >>\n\
             ] >> >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        let names: Vec<&str> = data.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["address", "address.city", "address.zip", "loose"]);
    }

    /// Table 249 states the precedence twice and this is it: `/Ff` replaces, and `/SetFf` and
    /// `/ClrFf` "shall be ignored if an `Ff` entry is present".
    #[test]
    fn ff_replaces_and_setff_modifies() {
        let document = fdf("1 0 obj\n<< /FDF << /Fields [\n\
             << /T (a) /Ff 4 /SetFf 8 /ClrFf 1 >>\n\
             << /T (b) /SetFf 8 /ClrFf 1 >>\n\
             << /T (c) >>\n\
             ] >> >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.fields[0].flags, FlagChange::Replace(4));
        assert_eq!(data.fields[0].flags.applied_to(3), 4, "/Ff replaces");
        assert_eq!(
            data.fields[1].flags,
            FlagChange::Modify { set: 8, clear: 1 }
        );
        assert_eq!(
            data.fields[1].flags.applied_to(3),
            10,
            "set before clear: (3 | 8) & !1"
        );
        assert_eq!(data.fields[2].flags, FlagChange::Unchanged);
        assert_eq!(data.fields[2].flags.applied_to(3), 3);
    }

    /// Table 246's `/Encoding` names four registered character sets this program carries no
    /// table for. The refusal is per *value*, so the rest of the file still imports — and a
    /// string carrying §7.9.2.2.1's U+FEFF marker is a text string the entry does not reach.
    #[test]
    fn a_registered_character_set_is_refused_by_name_and_a_marked_string_is_not() {
        let document = fdf("1 0 obj\n<< /FDF << /Encoding /Shift_JIS /Fields [\n\
             << /T (a) /V (bytes) >>\n\
             << /T (b) /V <FEFF0041> >>\n\
             ] >> >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.encoding, Encoding::Registered("Shift_JIS".to_owned()));
        assert_eq!(data.fields[0].value, None);
        assert_eq!(
            data.fields[0].owed,
            [
                "/T in a character set this program has no table for",
                "/V in a character set this program has no table for"
            ],
            "the name is encoded by the same entry the value is"
        );
        assert_eq!(
            data.fields[1].value.as_ref().and_then(Object::as_string),
            Some([254, 255, 0, 65].as_slice()),
            "a UTF-16BE marker is a text string whatever /Encoding says"
        );
        assert_eq!(
            data.fields[1].owed,
            ["/T in a character set this program has no table for"],
            "this field's value is decodable and its own name is not"
        );
    }

    /// Everything Table 246 states that this program does not act on is named rather than
    /// dropped, and a file carrying all of them still imports its fields.
    #[test]
    fn what_is_not_applied_is_named() {
        let document = fdf(
            "1 0 obj\n<< /FDF << /Fields [ << /T (a) /V (x) /RV (rich) >> ]\n\
             /Pages [ << /Templates [] >> ]\n\
             /JavaScript << /Before (app.alert\\(1\\)) >>\n\
             /Differences 2 0 R\n\
             /EmbeddedFDFs [ << /Type /Filespec /F (more.fdf) >> ]\n\
             /Annots [ << /Subtype /Text /Page 3 >> ]\n\
             >> >>\nendobj\n\
             2 0 obj\n<< /Length 3 >>\nstream\nabc\nendstream\nendobj",
        );
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.pages, 1);
        assert_eq!(data.annotations.len(), 1);
        assert_eq!(data.annotations[0].page, Some(3));
        assert_eq!(data.annotations[0].subtype.as_deref(), Some("Text"));
        assert_eq!(data.fields[0].owed, ["/RV: XFA rich text, excluded"]);
        assert_eq!(
            data.owed.len(),
            6,
            "/Pages, the Table 246 contradiction, /JavaScript, /EmbeddedFDFs, /Differences \
             and /Annots: {:?}",
            data.owed
        );
    }

    /// Table 246's `/Status` is a message from a server "that shall be displayed", and Table
    /// 246's `/ID` is what says whether this data belongs to the document in hand.
    #[test]
    fn the_file_says_which_document_it_is_for_and_what_the_server_said() {
        let document = fdf(
            "1 0 obj\n<< /FDF << /F (form.pdf) /ID [ <0102> <0304> ] /Status (Thank you) >> >>\nendobj",
        );
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.source.as_deref(), Some("form.pdf"));
        assert_eq!(data.status.as_deref(), Some("Thank you"));
        assert_eq!(
            data.identifier,
            Some([vec![1, 2], vec![3, 4]]),
            "two byte strings, §14.4's permanent identifier first"
        );
    }

    /// A `/Kids` cycle is a file a reader must survive, and Table 249's permission for an
    /// *indirect* child is what makes one writable.
    #[test]
    fn a_kids_cycle_terminates() {
        let document = fdf("1 0 obj\n<< /FDF << /Fields [ 2 0 R ] >> >>\nendobj\n\
             2 0 obj\n<< /T (a) /Kids [ 2 0 R ] >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.fields.len(), MAX_FIELD_DEPTH);
    }
}
