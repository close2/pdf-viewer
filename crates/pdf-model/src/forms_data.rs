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
//! Table 246's `/Pages` is *not* on that list: §12.7.8.3.3's templates name pages this document
//! already holds, under §12.7.7's `/Templates` name tree, so adding one costs a name lookup and
//! no page content at all. [`crate::view::ViewState`] holds the result and a viewer shows them
//! after the document's own pages.
//!
//! # An FDF file may carry FDF files, and each of them is a file
//!
//! Table 246's `/EmbeddedFDFs` is "[a]n array of file specifications … representing other FDF
//! files embedded within this one", an ordinary PDF 1.4 entry with no deprecation marker of any
//! kind. Each element is §7.11's specification over §7.11.4's embedded file stream, and what
//! comes out of the stream is an FDF file — so it is decoded and read by [`FormsData::read`],
//! whole, with its own `/Encoding` and its own `/Status`. [`FormsData::files`] is the nesting in
//! the order an import applies it and [`match_to_document`] is where that order becomes the
//! import.
//!
//! What ISO 32000-2 deprecates is the *encrypted* form — Table 247's `/EncryptionRevision`, whose
//! revision 1 is a 40-bit RC4 key derived from a padded user-supplied password — and an embedded
//! FDF stating that entry is refused by name on [`FormsData::owed`]. The prose above Table 247
//! is ambiguous about which of the two the deprecation governs; Errata Collection 3 Issue #173
//! settles it as the encryption's, and `doc/errata-read.md` records the strike. ADR 1185.
//!
//! # What is read and not applied, and why each
//!
//! Every one of these is *named* on [`FormsData::owed`] rather than skipped, which is principle
//! 3's requirement:
//!
//! - **Table 252's `/Rename`** decides which fully qualified name a template's fields answer to
//!   afterwards, and the clause says outright that the flag "does not define a renaming
//!   algorithm". It is read, and nothing here merges field trees for it to matter to.
//! - **`/JavaScript`** (Table 248) is on `CLAUDE.md`'s closed exclusion list.
//! - **`/Differences`** is the target document's own incremental updates, carried for a server;
//!   applying it would mean *writing* the target file, which principle 5 puts outside this
//!   project.
//! - **`/RV`** on a field: XFA rich text, which is excluded. `read_field` argues it. Table 249's
//!   `/IF`, `/AP`, `/A` and `/AA` are **not** among them: an icon fit dictionary states names,
//!   numbers and a boolean and nothing else, so it crosses whole and replaces Table 192's `/IF`
//!   on the widget (ADR 1186); the other three cross by [`carry`], which is the rule below
//!   (ADR 1223).
//! - **`/APRef` naming another file**: Table 253's `/F` makes the page a *document* named, which
//!   is §12.7.6.4's hazard — the bytes may come only from a directory a person supplied
//!   (ADR 1155) — and this crate has no filesystem and must not acquire one. A reference stating
//!   **no** `/F` is a different entry: Table 253 says "[i]f this entry is absent, it shall be
//!   assumed that the page resides in the associated PDF file", so the page is one *this*
//!   document holds under §12.7.7, and `crate::view::ViewState::import` makes the widget's
//!   appearance out of it (ADR 1235).
//!
//! # How an entry whose value lives in the other file crosses
//!
//! Table 249's `/AP` streams, its `/A` and `/AA` action dictionaries and Table 254's whole
//! annotation dictionaries all name objects of the **FDF** file, and §12.7.8.3.2 says their
//! values replace the target document's. A value that is an indirect reference into another file
//! is not a value this document can hold, so [`carry`] resolves every reference and copies what
//! it finds in its place: what crosses is a tree of direct objects that names nothing of the
//! other file. `pdf_syntax::Document` therefore stays immutable and singular — the interpreter
//! never holds two documents — and the copy lives in the log beside it
//! (`crate::view::ViewState`). ADR 1223.
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
pub(crate) const MAX_FIELDS: usize = 65536;

/// How deep the `/Kids` chain is followed.
///
/// §12.7.4.2's fully qualified name is built by concatenation, so a chain this long describes a
/// name no interface could show. The bound is also what makes a `/Kids` cycle terminate — Table
/// 249 permits a *direct* child, so a cycle needs an indirect one, and a file may write one.
pub(crate) const MAX_FIELD_DEPTH: usize = 64;

/// Most annotations listed from one FDF file's `/Annots`.
pub(crate) const MAX_ANNOTATIONS: usize = 4096;

/// Most Table 251 pages read, and most templates read from one of them.
///
/// A file adding more pages than this to a document is not composing a form.
const MAX_PAGES: usize = 4096;

/// Most FDF files read out of Table 246's `/EmbeddedFDFs`, counted over the whole nesting.
///
/// The standard states no number here and nothing it requires a reader to carry states one
/// either (trap 38), so this is a bound on *work* and says so: an FDF file that carries more
/// than this many other FDF files inside it has stopped describing one form's data.
const MAX_EMBEDDED_FILES: usize = 64;

/// How deep `/EmbeddedFDFs` is followed.
///
/// An embedded FDF is §7.11.4's stream *inside* the file that names it, so each level is strictly
/// contained in the one above and a cycle cannot be written the way a `/Kids` cycle can. What a
/// file can still write is a nesting whose decoded size grows at every level, which is why the
/// depth is bounded as well as the bytes.
const MAX_EMBEDDED_DEPTH: usize = 8;

/// Most bytes decoded out of embedded FDF streams, counted over the whole nesting.
///
/// The budget is shared across the nesting rather than applied per stream, because what a
/// decompression bomb costs is the total and not any one of its parts (`CLAUDE.md` principle 3).
/// Sixteen mebibytes is far past any form's field data and far short of what exhausts a reader.
const MAX_EMBEDDED_BYTES: usize = 16 * 1024 * 1024;

/// How many indirect references one carried entry may follow (ADR 1223).
///
/// Every reference [`carry`] follows costs one of these, so this is the total work a single
/// `/AP`, `/A`, `/AA` or annotation dictionary can ask of the copy — which is what makes a
/// `/Next` chain that loops, or a resource graph that fans out, terminate. The standard states no
/// number here and nothing it requires a reader to carry states one either (trap 38), so this is
/// a bound on *work* and says so.
const MAX_CARRIED_REFERENCES: usize = 4096;

/// How many bytes of stream data one carried entry may bring across.
///
/// A push-button's appearance is artwork; the images and fonts its `/Resources` name are the rest
/// of it. Sixteen mebibytes is far past any button and far short of what exhausts a reader —
/// [`MAX_EMBEDDED_BYTES`]'s reasoning, one clause along.
const MAX_CARRIED_BYTES: usize = 16 * 1024 * 1024;

/// How deep a carried object's own structure is followed.
///
/// The reference budget above bounds the *number* of objects; this bounds a single direct
/// structure, which an FDF file may nest as deeply as its parser allowed.
const MAX_CARRY_DEPTH: usize = 32;

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
    /// The version this file **conforms to**, which is the ranking §12.7.8.3.1's Table 245 states
    /// and not the entry alone.
    ///
    /// > If the header specifies a later version, or if this entry is absent, the document
    /// > conforms to the version specified in the header.
    ///
    /// So the answer is the later of the two, and [`Self::version`] alone is the entry rather
    /// than the meaning — a distinction that carries no modal verb and that a sweep reading verbs
    /// cannot see (ADR 0919). §7.7.2's `/Version` against §7.5.2's header is the identical
    /// construction one clause family over, so this **is** [`pdf_syntax::Document::version`]
    /// rather than a second copy of its arithmetic: one rule, one place, and an FDF written as
    /// §7.5.6's chain of updates reaches the version the last revision reached (ADR 1171).
    ///
    /// `None` where the file states neither, which is a file that never said.
    pub conforms_to: Option<pdf_syntax::Version>,
    /// Table 246's `/F`: "[t]he source file or target file", as the file spells it.
    ///
    /// A name for a person, not a path this program will open — the same position `/UF` and
    /// `/F` take in [`crate::attachment`]. A caller deciding whether this FDF belongs with the
    /// document it has open compares it, and [`Self::identifier`] is the stronger comparison.
    ///
    /// Read through [`crate::file_spec::FileSpec`], so §7.11.1's two forms both arrive and Table
    /// 43's `/UF` outranks its `/F` the way that table says. `None` for a specification that
    /// names no file at all, which Table 240 bit 14's embedded form is: there the file is the
    /// `/EF` stream, and [`Self::owed`] is where that is said.
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
    /// Table 246's `/Pages`: the template pages this file adds to the target document.
    pub pages: Vec<FdfPage>,
    /// Table 246's `/Target`, "[t]he name of a browser frame in which the underlying PDF
    /// document shall be opened".
    pub target: Option<String>,
    /// Table 246's `/EmbeddedFDFs`, each read as the FDF file it is, in the array's order.
    ///
    /// §12.7.8.3.1's Table 246 states the entry:
    ///
    /// > ( Optional; PDF 1.4 ) An array of file specifications (see 7.11, "File specifications")
    /// > representing other FDF files embedded within this one (7.11.4, "Embedded file streams").
    ///
    /// An FDF file rather than a payload, so it is read by [`FormsData::read`] like any other and
    /// arrives here whole — its own `/Encoding`, its own `/Status`, its own fields. That matters
    /// for more than tidiness: `/Encoding` is stated per file and decides how *that* file's field
    /// names and values become characters, so an embedded FDF decoded under the outer file's
    /// entry would be read in the wrong character set.
    ///
    /// [`Self::files`] is this file and everything under it in the order an import applies them,
    /// and [`match_to_document`] is where that order becomes the import. ADR 1185.
    pub embedded: Vec<FormsData>,
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
    /// Table 249's `/IF`, "[a]n icon fit dictionary … specifying how to display a button field's
    /// icon within the annotation rectangle of its widget annotation".
    ///
    /// Held as the dictionary the file states, because Table 250's four entries are names,
    /// numbers and a boolean — nothing in it reaches an object of the FDF file, so the dictionary
    /// carries across to the target document whole. That is what separates this entry from the
    /// other four Table 249 states beside it, and [`FdfField::owed`] names those. ADR 1186.
    pub icon_fit: Option<Dictionary>,
    /// Table 249's `/AP`, "[a]n appearance dictionary specifying the appearance of a push-button
    /// field", carried into this document's own space by [`carry`].
    ///
    /// The table's own exception is what makes this entry different from every other appearance
    /// dictionary a reader meets: its `/N`, `/R` and `/D` entries "shall all be streams", and
    /// those streams are objects of the **FDF** file. So the value that replaces the widget's
    /// `/AP` is a *copy*, whose `/Resources` and everything under them came across with it —
    /// after which nothing in it names the other file and the interpreter holds one document, as
    /// it always did. ADR 1223.
    ///
    /// `None` where the field states no `/AP`, and also where the copy exceeded its budget, which
    /// [`FdfField::owed`] names.
    pub appearance: Option<Dictionary>,
    /// Table 249's `/APRef`, as the Table 253 named page references its `/N`, `/R` and `/D` hold.
    ///
    /// The entry is "[a] dictionary holding references to external PDF files containing the pages
    /// to use for the appearances of a push-button field", "similar to an appearance dictionary …
    /// except that the values of the N, R , and D entries shall all be named page reference
    /// dictionaries". So what is read here is a name per appearance state, and resolving it is
    /// `crate::view::ViewState::import`'s — the name is looked up in the **target** document's
    /// §12.7.7 trees, which this reader of the FDF file has not got.
    ///
    /// Empty where the field states no `/APRef`, and **also where it states an `/AP`**: Table 249
    /// says "[t]his entry shall be ignored if an AP entry is present." ADR 1235.
    pub appearance_reference: Vec<(&'static str, crate::named_page::Reference)>,
    /// Table 249's `/A` and `/AA`, carried by [`carry`] and held under those two key names.
    ///
    /// A dictionary rather than two fields because that is exactly what §12.6.3 and Table 197
    /// read: `crate::action::for_annotation` takes an *annotation dictionary* and applies the one
    /// precedence rule between the two entries, so what an import has to supply is a dictionary
    /// stating whichever of them the FDF file stated. §12.7.8.3.2's replacing sentence does the
    /// rest — an imported `/A` replaces the widget's `/A`, an imported `/AA` replaces its `/AA`,
    /// and an entry the file does not state leaves the widget's standing. ADR 1223.
    ///
    /// `None` where the field states neither, and where the copy exceeded its budget.
    pub actions: Option<Dictionary>,
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

/// One page an FDF file adds to the target document. Table 251.
///
/// §12.7.8.3.3 makes this a *composition*: a page is made of one or more templates, each of
/// which is a page the target document already holds under a name (§12.7.7). So an FDF file
/// carrying pages does not carry any page content — it names pages that are already there.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfPage {
    /// Table 251's `/Templates`, "[r]equired", the named pages that serve as templates on it.
    pub templates: Vec<FdfTemplate>,
    /// Whether Table 251's optional `/Info` page information dictionary is present.
    ///
    /// The table says only that it "shall contain additional information about the page" and
    /// names not one entry, so there is nothing to read and something to say.
    pub has_info: bool,
}

/// One template on an FDF page. Table 252.
#[derive(Debug, Clone, PartialEq)]
pub struct FdfTemplate {
    /// Table 252's `/TRef`, "[r]equired", the named page reference specifying its location.
    pub reference: crate::named_page::Reference,
    /// Table 252's `/Fields`, "[a]n array of references to FDF field dictionaries … describing
    /// the root fields that shall be imported (those with no ancestors in the field hierarchy)".
    ///
    /// *Imported*, which is into the target document — the same act §12.7.8.3.2 defines and the
    /// same §12.7.4.2 names it matches on. Whether they may be applied is Table 252's `/Rename`
    /// to decide, not this entry.
    pub fields: Vec<FdfField>,
    /// Table 252's `/Rename`, **default `true`**.
    ///
    /// The flag decides what happens to a template field whose name a field of the target
    /// document already has, and a template added by reference to a page this document already
    /// holds makes that every one of them. §12.7.8.3.3's Table 252 states both branches:
    ///
    /// > If this flag is true , fields with such conflicting names shall be renamed to guarantee
    /// > their uniqueness. If false , the fields shall not be renamed; this results in multiple
    /// > fields with the same name in the target document. Each time the FDF file provides
    /// > attributes for a given field name, all fields with that name shall be updated.
    ///
    /// The `false` branch is exactly what [`crate::view::ViewState::import`] does — one value per
    /// name, applied to every widget of that name — and is applied. The `true` branch asks for
    /// *new fields* under names this document does not have, which is a field tree this program
    /// cannot write: `CLAUDE.md` rule 1 makes the document immutable and an edit a log beside it.
    /// So a `true` is refused by name rather than applied as though it were a `false`, which
    /// would put the template's values on fields the clause says to leave alone.
    pub rename: bool,
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
    /// The annotation dictionary, **carried** into this document's own space by [`carry`].
    ///
    /// §12.7.8.3.4 states one entry of its own and everything else in the dictionary is §12.5's,
    /// read by the clauses that read any annotation — so what stops one being drawn is only that
    /// its `/AP` and its resources are objects of the FDF file. [`carry`] is that answer: the
    /// copy names nothing of the other file, and `crate::view::ViewState::import` puts it on the
    /// page Table 254's `/Page` states. ADR 1223.
    ///
    /// `None` where the copy exceeded its budget, which [`FormsData::owed`] names.
    pub dictionary: Option<Dictionary>,
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
        Self::read_within(document, &mut Budget::new(), 0)
    }

    /// The same, under the budget Table 246's `/EmbeddedFDFs` nesting is read within.
    ///
    /// `depth` is how many `/EmbeddedFDFs` arrays were followed to reach this file, so the
    /// outermost is 0. The budget is one object for the whole nesting — see [`Budget`].
    fn read_within(
        document: &Document,
        budget: &mut Budget,
        depth: usize,
    ) -> Result<Self, FormsDataError> {
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

        let pages = read_pages(document, &document.get_key(&fdf, "Pages"), &encoding);
        if pages
            .iter()
            .flat_map(|page| &page.templates)
            .any(|template| !template.fields.is_empty())
        {
            owed.push("/Fields on a template: fields of the template's own field hierarchy");
        }
        if pages.iter().any(|page| page.has_info) {
            owed.push("/Info on a page: a dictionary Table 251 names no entry of");
        }
        // Table 246 states two exclusions between these three entries, and both are rules about
        // a *writer*: "[t]his entry and the Pages entry shall not both be present" of `/Fields`
        // and of `/Status`. A reader meeting a file that breaks them has data it can use and no
        // clause telling it which to prefer, so both are read and the contradiction is named.
        if !pages.is_empty() && !fields.is_empty() {
            owed.push("/Fields and /Pages are both present, which Table 246 forbids");
        }

        // Table 246 types `/F` as a "file specification", and §7.11.1 gives that two forms:
        // "either a string or a dictionary". Read through [`crate::file_spec::FileSpec`] so that
        // both arrive, which matters because this program writes the second one itself — Table
        // 240 bit 14's `EmbedForm` makes `/F` "a file specification containing an embedded file
        // stream representing the PDF file from which the FDF is being submitted", and a reader
        // that saw only the string form would read that file as naming no source at all.
        let source = crate::file_spec::FileSpec::parse(document, &document.get_key(&fdf, "F"));

        let annotations = read_annotations(document, &document.get_key(&fdf, "Annots"));
        let embedded = read_embedded(document, &fdf, budget, depth, &mut owed);
        for (key, why) in [
            (
                "JavaScript",
                "/JavaScript: document-level scripts, excluded",
            ),
            (
                "Differences",
                "/Differences: the target document's own incremental updates",
            ),
        ] {
            if !document.get_key(&fdf, key).is_null() {
                owed.push(why);
            }
        }
        // §12.7.8.3.4's own requirement is Table 254's `/Page`, "[t]he ordinal page number on
        // which this annotation shall appear", and an annotation stating none names no page to
        // appear on. One whose dictionary the copy budget stopped states nothing to draw. Both
        // are named; the rest are placed by `crate::view::ViewState::import` (ADR 1223).
        if annotations
            .iter()
            .any(|annotation| annotation.page.is_none())
        {
            owed.push("/Annots: an annotation with no Table 254 /Page, which names no page");
        }
        if annotations
            .iter()
            .any(|annotation| annotation.dictionary.is_none())
        {
            owed.push("/Annots: an annotation larger than this reader copies out of an FDF file");
        }
        // §7.11.4's embedded file, which for an FDF is the whole source document: read as a
        // statement that it is there rather than extracted, because nothing here opens a second
        // document out of the first.
        if source.as_ref().is_some_and(|spec| spec.embedded) {
            owed.push("/F: a file specification carrying the source document as an embedded file");
        }

        let stated = document.get_key(&catalog, "Version");
        let stated = stated.as_name();
        Ok(Self {
            version: stated.map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
            conforms_to: document.version(),
            source: source
                .as_ref()
                .and_then(crate::file_spec::FileSpec::display_name),
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
            embedded,
            owed,
        })
    }

    /// This file and every file embedded in it, in the order an import applies them.
    ///
    /// Outermost first, then each of Table 246's `/EmbeddedFDFs` in the array's own order and
    /// everything under it — which is what "in turn" means for a nesting, and what
    /// [`match_to_document`] walks. A later file's statement about a widget is the later
    /// statement, exactly as a second import of a second file would be.
    #[must_use]
    pub fn files(&self) -> Vec<&Self> {
        let mut out = Vec::new();
        self.collect_files(&mut out);
        out
    }

    /// [`Self::files`]'s walk, which is pre-order and nothing else.
    fn collect_files<'a>(&'a self, into: &mut Vec<&'a Self>) {
        into.push(self);
        for embedded in &self.embedded {
            embedded.collect_files(into);
        }
    }

    /// Every `/Status` this file and its embedded files state, in [`Self::files`]'s order.
    ///
    /// Table 246 makes the entry "[a] status string that shall be displayed", and an embedded FDF
    /// is an FDF — so a status inside one is a message to the person at the screen on the same
    /// sentence's authority, and a caller that displayed only the outermost would have dropped it.
    #[must_use]
    pub fn statuses(&self) -> Vec<&str> {
        self.files()
            .into_iter()
            .filter_map(|file| file.status.as_deref())
            .collect()
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

/// What a whole `/EmbeddedFDFs` nesting is allowed to cost, in files and in decoded bytes.
///
/// One object for the whole nesting rather than one per level: a file that embeds two FDFs each
/// embedding two more has written four decodes, and a per-level bound would let it write as many
/// as it liked. `CLAUDE.md` principle 3 — "[m]emory safety is not enough" — is why the bytes are
/// counted at all, since every one of them is a decode this reader performs on the file's say-so.
struct Budget {
    /// How many more FDF files may be read out of embedded streams.
    files: usize,
    /// How many more decoded bytes may be taken out of embedded streams.
    bytes: usize,
}

impl Budget {
    /// A whole nesting's allowance, as [`MAX_EMBEDDED_FILES`] and [`MAX_EMBEDDED_BYTES`] state it.
    const fn new() -> Self {
        Self {
            files: MAX_EMBEDDED_FILES,
            bytes: MAX_EMBEDDED_BYTES,
        }
    }
}

/// Table 246's `/EmbeddedFDFs`, read as the FDF files the entry says they are.
///
/// §12.7.8.3.1's Table 246:
///
/// > ( Optional; PDF 1.4 ) An array of file specifications (see 7.11, "File specifications")
/// > representing other FDF files embedded within this one (7.11.4, "Embedded file streams").
///
/// Three readings are stacked and each is the cell's own: §7.11's file specification, §7.11.4's
/// embedded file stream under its `/EF`, and §12.7.8's FDF file in the bytes that come out. So
/// each element is decoded and then handed to [`FormsData::read_within`], which is the same
/// function that read the file naming it — an embedded FDF is a file, not a fragment.
///
/// **The entry is not deprecated and this reader owes it.** Table 246's cell carries no
/// deprecation marker; what ISO 32000-2 deprecates is the *encrypted* form, which Table 247
/// states as "(Required if the FDF file is encrypted; deprecated in PDF 2.0)" and which the
/// paragraph above that table leaves ambiguous. Errata Collection 3 Issue #173 rewrites that
/// opening so the participle governs FDF file encryption, and `doc/errata-read.md` records it.
/// ADR 1185.
///
/// **An encrypted embedded FDF is refused by name**, which is the whole of what stays unbuilt
/// here: Table 247's revision 1 is a 40-bit RC4 key derived from a padded user-supplied password,
/// and this program has neither the password nor a reason to carry a deprecated key derivation.
/// The refusal is on `owed`, so a person is told which file went unread rather than shown a form
/// silently missing its values (trap 5).
fn read_embedded(
    document: &Document,
    fdf: &Dictionary,
    budget: &mut Budget,
    depth: usize,
    owed: &mut Vec<&'static str>,
) -> Vec<FormsData> {
    let entry = document.get_key(fdf, "EmbeddedFDFs");
    let Some(items) = entry.as_array() else {
        return Vec::new();
    };
    if depth >= MAX_EMBEDDED_DEPTH {
        push_once(
            owed,
            "/EmbeddedFDFs: a nesting deeper than this reader follows",
        );
        return Vec::new();
    }
    let mut out = Vec::new();
    for item in items {
        if budget.files == 0 {
            push_once(
                owed,
                "/EmbeddedFDFs: more embedded files than this reader reads",
            );
            break;
        }
        let resolved = document.resolve(item);
        let Some(specification) = resolved.as_dict() else {
            continue;
        };
        // §7.11.4.2's `/EF`, whose keys are the same five §7.11.4.1 gives a file specification and
        // which `crate::attachment::read` takes in the same order: the Unicode name first,
        // because Table 43 ranks `/UF` above `/F`.
        let files = document.get_key(specification, "EF");
        let Some(files) = files.as_dict() else {
            push_once(
                owed,
                "/EmbeddedFDFs: a file specification naming a file outside this one",
            );
            continue;
        };
        let Some(stream) = ["UF", "F", "DOS", "Mac", "Unix"]
            .into_iter()
            .find_map(|key| document.get_key(files, key).as_stream().cloned())
        else {
            push_once(
                owed,
                "/EmbeddedFDFs: an /EF dictionary carrying no embedded file stream",
            );
            continue;
        };
        if !document
            .get_key(&stream.dict, "EncryptionRevision")
            .is_null()
        {
            push_once(
                owed,
                "/EmbeddedFDFs: an encrypted FDF, whose Table 247 key derivation is deprecated",
            );
            continue;
        }
        let Some(bytes) = document.decoded_stream_data(&stream) else {
            push_once(
                owed,
                "/EmbeddedFDFs: an embedded file stream this reader could not decode",
            );
            continue;
        };
        if bytes.len() > budget.bytes {
            push_once(
                owed,
                "/EmbeddedFDFs: more embedded bytes than this reader decodes",
            );
            break;
        }
        budget.bytes = budget.bytes.saturating_sub(bytes.len());
        budget.files = budget.files.saturating_sub(1);
        let Ok(inner) = Document::open(bytes.to_vec()) else {
            push_once(
                owed,
                "/EmbeddedFDFs: an embedded file this reader could not open",
            );
            continue;
        };
        match FormsData::read_within(&inner, budget, depth.saturating_add(1)) {
            Ok(data) => {
                // The embedded file's own refusals are the outer file's to report, because the
                // one list a caller prints is this one — `viewer_core` puts every entry of it in
                // front of a person, and a refusal held one level down would be a silence.
                for why in &data.owed {
                    push_once(owed, why);
                }
                out.push(data);
            }
            Err(_) => push_once(
                owed,
                "/EmbeddedFDFs: an embedded file that is not forms data",
            ),
        }
    }
    out
}

/// Adds a reason to `owed` unless it is already there.
///
/// A nesting can reach the same refusal at every level, and a person reading a status line is
/// owed the sentence once rather than once per file.
fn push_once(owed: &mut Vec<&'static str>, why: &'static str) {
    if !owed.contains(&why) {
        owed.push(why);
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

/// What one carried entry is allowed to cost, in references followed and in stream bytes.
///
/// One object per entry that crosses, so an FDF file carrying a hundred push-button appearances
/// pays [`MAX_CARRIED_REFERENCES`] for each rather than a hundredth of it — the unit is what a
/// reader would have to refuse, and refusing one button's artwork while drawing the next is the
/// honest answer. See [`carry`].
struct Carried {
    /// How many more indirect references may be followed.
    references: usize,
    /// How many more bytes of stream data may be brought across.
    bytes: usize,
}

impl Carried {
    /// One entry's allowance, as [`MAX_CARRIED_REFERENCES`] and [`MAX_CARRIED_BYTES`] state it.
    const fn new() -> Self {
        Self {
            references: MAX_CARRIED_REFERENCES,
            bytes: MAX_CARRIED_BYTES,
        }
    }
}

/// Copies one object of an FDF file into an object that names nothing of that file.
///
/// **This is the rule the four Table 249 entries that live in the other file cross by**, and it
/// is what §12.7.8.3.2's own sentence asks for once the entry's value is not a name or a number:
///
/// > Unless otherwise indicated in the table, importing a field causes the values of the entries
/// > in the FDF field dictionary to replace those of the corresponding entries in the field with
/// > the same fully qualified name in the target document.
///
/// *Replace those of the corresponding entries* is a statement about **values**, and a value that
/// is an indirect reference into the FDF file is not a value the target document can hold. So
/// every reference is resolved and its value copied in its place: what comes back is a tree of
/// direct objects, self-contained, which the interpreter reads with no knowledge that a second
/// file ever existed. That is the whole of the design — `pdf_syntax::Document` stays immutable
/// and singular, and the copy lives in the log beside it (`crate::view::ViewState`). ADR 1223.
///
/// `None` where the copy would exceed [`Carried::new`]'s allowance or [`MAX_CARRY_DEPTH`], which
/// is a refusal of the **whole** entry rather than half of it: half an appearance is a button
/// drawn wrong, and a caller that gets `None` names the entry on `owed` instead (trap 5). A
/// `/Next` chain that loops and a resource graph that fans out both end here, because each hop
/// spends one of the references.
///
/// **A stream whose bytes could not be decrypted is refused rather than carried empty**, for the
/// same reason [`pdf_syntax::Document::decoded_stream_data`] refuses one: a stream that silently
/// became empty draws nothing and reports nothing.
fn carry(
    document: &Document,
    value: &Object,
    budget: &mut Carried,
    depth: usize,
) -> Option<Object> {
    if depth >= MAX_CARRY_DEPTH {
        return None;
    }
    match value {
        Object::Reference(_) => {
            budget.references = budget.references.checked_sub(1)?;
            // §7.3.10: "An indirect reference to an undefined object shall not be considered an
            // error by a PDF processor; it shall be treated as a reference to the null object."
            // `Document::resolve` is that sentence, so a dangling reference crosses as null.
            let resolved = document.resolve(value);
            carry(document, &resolved, budget, depth.saturating_add(1))
        }
        Object::Array(items) => items
            .iter()
            .map(|item| carry(document, item, budget, depth.saturating_add(1)))
            .collect::<Option<Vec<Object>>>()
            .map(Object::Array),
        Object::Dictionary(dict) => {
            carry_dictionary(document, dict, budget, depth).map(Object::Dictionary)
        }
        Object::Stream(stream) => {
            if stream.decryption_failed {
                return None;
            }
            budget.bytes = budget.bytes.checked_sub(stream.data.len())?;
            let dict = carry_dictionary(document, &stream.dict, budget, depth)?;
            Some(Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
                dict,
                data: std::sync::Arc::clone(&stream.data),
                decryption_failed: false,
            })))
        }
        other => Some(other.clone()),
    }
}

/// [`carry`] over a dictionary's values, keys unchanged.
///
/// A stream's `/Length` is the entry this matters most for: §7.3.8.2 lets a writer state it as an
/// indirect reference, and a carried stream whose length still named an object of the FDF file
/// would decode to nothing in the document it crossed into.
fn carry_dictionary(
    document: &Document,
    dict: &Dictionary,
    budget: &mut Carried,
    depth: usize,
) -> Option<Dictionary> {
    let mut out = Dictionary::new();
    for (key, value) in dict.iter() {
        out.insert(
            key.clone(),
            carry(document, value, budget, depth.saturating_add(1))?,
        );
    }
    Some(out)
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
    // Table 249's `/AP`: "[a]n appearance dictionary specifying the appearance of a push-button
    // field", whose `/N`, `/R` and `/D` "shall all be streams" — streams that are objects of the
    // FDF file. [`carry`] is how they cross, so the entry is applied rather than named; what is
    // named is a copy the budget stopped. ADR 1223.
    let stated_appearance = document.get_key(field, "AP");
    let mut appearance = None;
    if !stated_appearance.is_null() {
        appearance = carry(document, &stated_appearance, &mut Carried::new(), 0)
            .and_then(|carried| carried.as_dict().cloned());
        if appearance.is_none() {
            owed.push("/AP: an appearance larger than this reader copies out of an FDF file");
        }
    }
    // `/A` and `/AA` cross by the same rule and into one dictionary, because §12.6.3's Table 197
    // states a precedence *between* them that only a reader holding both can apply.
    let mut actions = Dictionary::new();
    for key in ["A", "AA"] {
        let stated = document.get_key(field, key);
        if stated.is_null() {
            continue;
        }
        match carry(document, &stated, &mut Carried::new(), 0) {
            Some(carried) => {
                actions.insert(pdf_syntax::Name::new(key.as_bytes()), carried);
            }
            None if key == "A" => {
                owed.push("/A: an action chain longer than this reader copies out of an FDF file");
            }
            None => {
                owed.push("/AA: a trigger event this reader could not copy out of an FDF file");
            }
        }
    }
    // Table 249's `/APRef`, read only where the table lets it be reached: "[t]his entry shall be
    // ignored if an AP entry is present." Its `/N`, `/R` and `/D` are Table 253 named page
    // references, whose names belong to the *target* document's §12.7.7 trees — so what is read
    // here is the reference and the resolving is `crate::view::ViewState::import`'s. ADR 1235.
    let mut appearance_reference = Vec::new();
    if stated_appearance.is_null() {
        let references = document.get_key(field, "APRef");
        if let Some(references) = references.as_dict() {
            for key in APPEARANCE_STATES {
                let state = document.get_key(references, key);
                let Some(state) = state.as_dict() else {
                    continue;
                };
                if let Some(reference) = crate::named_page::Reference::read(document, state) {
                    appearance_reference.push((key, reference));
                }
            }
        }
        if !references.is_null() && appearance_reference.is_empty() {
            owed.push(
                "/APRef: no /N, /R or /D holding a Table 253 named page reference with a /Name",
            );
        }
    }
    // `/RV` is a rich text string, whose formatting no part of this tree applies — §12.7.4.3's
    // own departure, reported on the field it is drawn for rather than here — so importing it
    // would change nothing a reader sees. ADRs 1186, 1197, 1223.
    if !document.get_key(field, "RV").is_null() {
        owed.push("/RV: a rich text string whose XFA 3.3 formatting §12.7.4.3 does not apply here");
    }
    FdfField {
        name,
        value,
        flags: FlagChange::read(document, field, "Ff", "SetFf", "ClrFf"),
        annotation_flags: FlagChange::read(document, field, "F", "SetF", "ClrF"),
        options: options(document, field, encoding),
        icon_fit: document.get_key(field, "IF").as_dict().cloned(),
        appearance,
        appearance_reference,
        actions: (!actions.is_empty()).then_some(actions),
        owed,
    }
}

/// Table 170's three appearance states, which Table 249's `/AP` and `/APRef` both key on.
///
/// In §12.5.5's own order of preference — the normal appearance first, which is the one a widget
/// that is neither under the pointer nor pressed is drawn from.
const APPEARANCE_STATES: [&str; 3] = ["N", "R", "D"];

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

/// Table 246's `/Pages`, and Tables 251, 252 and 253 under it.
fn read_pages(document: &Document, entry: &Object, encoding: &Encoding) -> Vec<FdfPage> {
    let resolved = document.resolve(entry);
    let Some(items) = resolved.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .take(MAX_PAGES)
        .filter_map(|item| {
            let resolved = document.resolve(item);
            let page = resolved.as_dict()?;
            let templates = document.get_key(page, "Templates");
            let templates = templates.as_array().unwrap_or_default();
            Some(FdfPage {
                templates: templates
                    .iter()
                    .take(MAX_PAGES)
                    .filter_map(|item| {
                        let resolved = document.resolve(item);
                        let template = resolved.as_dict()?;
                        let target = document.get_key(template, "TRef");
                        let reference =
                            crate::named_page::Reference::read(document, target.as_dict()?)?;
                        let mut fields = Vec::new();
                        read_fields(
                            document,
                            &document.get_key(template, "Fields"),
                            "",
                            0,
                            encoding,
                            &mut fields,
                        );
                        Some(FdfTemplate {
                            reference,
                            fields,
                            // Table 252 states the default and it is `true`, which is the shape
                            // this project's habits warn about: a parameter whose default is the
                            // behaviour nobody implemented.
                            rename: !matches!(
                                document.get_key(template, "Rename"),
                                Object::Boolean(false)
                            ),
                        })
                    })
                    .collect(),
                has_info: !document.get_key(page, "Info").is_null(),
            })
        })
        .collect()
}

/// Table 254's annotations, each with the page it says it belongs to and its dictionary carried.
///
/// The `/Page` and `/Subtype` are read from the FDF file's own dictionary — both are an integer
/// and a name, so neither can be a reference into anything — and the dictionary that crosses is
/// [`carry`]'s copy. An annotation whose copy the budget stopped keeps its page and its subtype
/// and states no dictionary, which is what `FormsData::owed` then names. ADR 1223.
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
            let stated = resolved.as_dict()?;
            Some(FdfAnnotation {
                page: document
                    .get_key(stated, "Page")
                    .as_integer()
                    .and_then(|page| usize::try_from(page).ok()),
                subtype: document
                    .get_key(stated, "Subtype")
                    .as_name()
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
                dictionary: carry_dictionary(document, stated, &mut Carried::new(), 0),
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
    /// Table 249's `/IF`, which replaces Table 192's `/IF` in the widget's `/MK`.
    ///
    /// Both entries name the same Table 250 icon fit dictionary — the table is printed under
    /// §12.7.8.3.2 and §12.5.6.19's Table 192 points at it — so importing one is §12.7.8.3.2's
    /// replacing sentence applied to "the corresponding entrie[s] in the field with the same
    /// fully qualified name in the target document", and the icon it fits is the widget's own
    /// `/MK /I`. ADR 1186.
    pub icon_fit: Option<Dictionary>,
    /// Table 249's `/AP`, carried, which replaces the widget's own appearance dictionary.
    ///
    /// §12.7.8.3.2's replacing sentence over Table 170's entry: an FDF field's `/AP` and a widget
    /// annotation's `/AP` are the same appearance dictionary — the table says so, "as shown in
    /// "Table 170 - Entries in an appearance dictionary"" — so this is the corresponding entry
    /// and it replaces. The streams in it are the FDF producer's own marks, copied rather than
    /// referenced. ADR 1223.
    pub appearance: Option<Dictionary>,
    /// Table 249's `/APRef`, as the named pages it holds per appearance state.
    ///
    /// Read from the FDF file and resolved against the **target** document, which is where
    /// §12.7.7's name trees are: [`crate::view::ViewState::import`] turns each name that resolves
    /// into a form `XObject` made from that page ([`crate::named_page::page_as_form`]) and writes
    /// the result into [`Self::appearance`], so that a resolved `/APRef` and a stated `/AP` reach
    /// the widget by one route. What is left here afterwards is what did not resolve, which the
    /// import names. ADR 1235.
    pub appearance_reference: Vec<(&'static str, crate::named_page::Reference)>,
    /// Table 249's `/A` and `/AA`, carried, under those two key names.
    ///
    /// Read by `crate::action::for_annotation` as though it were the widget's own dictionary,
    /// which is what lets Table 197's precedence between the two apply unchanged. ADR 1223.
    pub actions: Option<Dictionary>,
}

/// Pairs an FDF file's fields with a target document's widgets, by fully qualified name.
///
/// The names on both sides are §12.7.4.2's, which is the whole of §12.7.8.3.2's matching rule.
/// A field the target document does not have is *not* an error and not silently dropped: it is
/// returned as its own list, because a file naming fields a form has not got is either the wrong
/// FDF for this document or a form that has changed, and a caller should be able to say which.
///
/// **Table 246's `/EmbeddedFDFs` are matched here too**, in [`FormsData::files`]'s order: this
/// file's fields first, then each embedded file's, and the pairs come back in that order so that
/// a caller applying them in turn leaves the later file's statement about a widget standing.
/// That is what importing a nesting *is* — each embedded file is an FDF, and the clause's
/// replacing sentence is about a field rather than about a file. ADR 1185.
#[must_use]
pub fn match_to_document(
    data: &FormsData,
    widgets: &std::collections::BTreeMap<String, Vec<ObjectId>>,
) -> (Vec<(ObjectId, Import)>, Vec<String>) {
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    for file in data.files() {
        let (found, missing) = match_fields(&file.fields, widgets);
        matched.extend(found);
        unmatched.extend(missing);
    }
    (matched, unmatched)
}

/// The same pairing over one list of fields, which §12.7.8 states in two places.
///
/// Table 246's `/Fields` is a file's; Table 252's is a template's, "the root fields that shall be
/// imported". Both are Table 249's dictionaries matched by §12.7.4.2's name, so both go through
/// this rather than through two readings that could disagree.
#[must_use]
pub fn match_fields(
    fields: &[FdfField],
    widgets: &std::collections::BTreeMap<String, Vec<ObjectId>>,
) -> (Vec<(ObjectId, Import)>, Vec<String>) {
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();
    for field in fields {
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
                    icon_fit: field.icon_fit.clone(),
                    appearance: field.appearance.clone(),
                    appearance_reference: field.appearance_reference.clone(),
                    actions: field.actions.clone(),
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

    /// §12.7.8.3.1's Table 245 makes `/Version` a *ranking* against the header, and the sentence
    /// that says so carries no modal verb.
    ///
    /// > If the header specifies a later version, or if this entry is absent, the document
    /// > conforms to the version specified in the header.
    ///
    /// Three cases, because the rule has three answers and reading the entry alone gets two of
    /// them wrong: an entry later than the header wins, an entry *earlier* than the header loses
    /// to it, and a file stating no entry conforms to its header. The entry is still reported as
    /// the file spells it — the ranking is a second answer, not a replacement (ADR 0919).
    #[test]
    fn the_version_an_fdf_conforms_to_is_the_later_of_its_header_and_its_entry() {
        let read = |catalog: &str| -> FormsData {
            let document = Document::open(
                format!("%FDF-1.2\n1 0 obj\n<< /FDF << >> {catalog} >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n")
                    .into_bytes(),
            )
            .expect("an FDF file");
            FormsData::read(&document).expect("an FDF catalog")
        };

        let later = read("/Version /1.4");
        assert_eq!(later.version.as_deref(), Some("1.4"));
        assert_eq!(
            later.conforms_to,
            Some(pdf_syntax::Version { major: 1, minor: 4 }),
            "the entry is later than the %FDF-1.2 header, so it is what the file conforms to"
        );

        let earlier = read("/Version /1.1");
        assert_eq!(
            earlier.conforms_to,
            Some(pdf_syntax::Version { major: 1, minor: 2 }),
            "the header is later, and the clause gives it to the header"
        );

        let absent = read("");
        assert_eq!(absent.version, None);
        assert_eq!(
            absent.conforms_to,
            Some(pdf_syntax::Version { major: 1, minor: 2 }),
            "\"or if this entry is absent, the document conforms to the version specified in the \
             header\""
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

    /// Table 249's `/A` and `/AA` cross as *values*, which is what §12.7.8.3.2's replacing
    /// sentence asks for: an action dictionary whose own `/Next` chain is indirect in the FDF
    /// file arrives here with every hop copied in place, so nothing in it names an object of
    /// that file and `crate::action::read` walks it without ever resolving a reference. ADR 1223.
    #[test]
    fn an_action_chain_crosses_with_every_hop_copied_in_place() {
        let document = fdf("1 0 obj\n<< /FDF << /Fields [ << /T (a) /A 2 0 R \
             /AA << /Fo 4 0 R >> >> ] >> >>\nendobj\n\
             2 0 obj\n<< /S /ResetForm /Next 3 0 R >>\nendobj\n\
             3 0 obj\n<< /S /Named /N /NextPage >>\nendobj\n\
             4 0 obj\n<< /S /Named /N /FirstPage >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        let actions = data.fields[0]
            .actions
            .as_ref()
            .expect("both entries crossed");
        assert!(data.fields[0].owed.is_empty(), "{:?}", data.fields[0].owed);
        assert!(
            !names_an_object(&Object::Dictionary(actions.clone())),
            "the copy names nothing of the FDF file: {actions:?}"
        );
        // The chain is what a `/Next` is *for*, so the copy has to have followed it: reading the
        // outermost action alone would pass a test that only checked for no references.
        let chain = crate::action::read(&document, actions.get("A").expect("/A crossed"));
        assert_eq!(chain.len(), 2, "{chain:?}");
        let triggered = crate::action::read(&document, actions.get("AA").expect("/AA crossed"));
        assert!(triggered.is_empty(), "an /AA is a dictionary of triggers");
    }

    /// A `/Next` chain that loops ends at the budget rather than running forever, and the whole
    /// entry is refused by name rather than half-copied (trap 5).
    #[test]
    fn an_action_chain_that_loops_is_refused_by_name() {
        let document = fdf(
            "1 0 obj\n<< /FDF << /Fields [ << /T (a) /A 2 0 R >> ] >> >>\nendobj\n\
             2 0 obj\n<< /S /Named /N /NextPage /Next 3 0 R >>\nendobj\n\
             3 0 obj\n<< /S /Named /N /FirstPage /Next 2 0 R >>\nendobj",
        );
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert!(data.fields[0].actions.is_none());
        assert_eq!(
            data.fields[0].owed,
            ["/A: an action chain longer than this reader copies out of an FDF file"]
        );
    }

    /// Whether any part of a carried value still refers to an object of the file it came from.
    ///
    /// The property [`carry`] exists for, asserted directly rather than through what happens to
    /// be drawn: a copy that left one reference behind would resolve against the *target*
    /// document's object of that number and draw whatever happened to be there.
    fn names_an_object(value: &Object) -> bool {
        match value {
            Object::Reference(_) => true,
            Object::Array(items) => items.iter().any(names_an_object),
            Object::Dictionary(dict) => dict.iter().any(|(_, entry)| names_an_object(entry)),
            Object::Stream(stream) => stream.dict.iter().any(|(_, entry)| names_an_object(entry)),
            _ => false,
        }
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
        assert_eq!(data.pages.len(), 1);
        assert_eq!(data.annotations.len(), 1);
        assert_eq!(data.annotations[0].page, Some(3));
        assert_eq!(data.annotations[0].subtype.as_deref(), Some("Text"));
        assert_eq!(
            data.fields[0].owed,
            ["/RV: a rich text string whose XFA 3.3 formatting §12.7.4.3 does not apply here"]
        );
        // Named one by one rather than counted: an assertion on a length would accept any five
        // sentences, including five of the wrong ones (trap 27).
        assert_eq!(
            data.owed,
            [
                "/Fields and /Pages are both present, which Table 246 forbids",
                "/EmbeddedFDFs: a file specification naming a file outside this one",
                "/JavaScript: document-level scripts, excluded",
                "/Differences: the target document's own incremental updates",
            ]
        );
    }

    /// Tables 251, 252 and 253, and the one entry of the three the clause says nobody can
    /// implement: Table 252's `/Rename` "does not define a renaming algorithm", and its default
    /// is `true`.
    #[test]
    fn a_template_page_names_a_page_this_document_already_holds() {
        let document = fdf("1 0 obj\n<< /FDF << /Pages [\n\
             << /Templates [ << /TRef << /Name (blank) >> >> \
             << /TRef << /Name (letterhead) /F (library.pdf) >> /Rename false >> ] /Info << >> >>\n\
             ] >> >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.pages.len(), 1);
        assert!(data.pages[0].has_info);
        let templates = &data.pages[0].templates;
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].reference.name, "blank");
        assert_eq!(templates[0].reference.file, None, "this document");
        assert!(templates[0].rename, "Table 252's default is true");
        assert_eq!(templates[1].reference.file.as_deref(), Some("library.pdf"));
        assert!(!templates[1].rename);
        assert_eq!(
            data.owed,
            ["/Info on a page: a dictionary Table 251 names no entry of"]
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
        let document = fdf("1 0 obj\n<< /FDF << /Fields [ 2 0 R ] >>\n>>\nendobj\n\
             2 0 obj\n<< /T (a) /Kids [ 2 0 R ] >>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.fields.len(), MAX_FIELD_DEPTH);
    }

    /// The same, from object bodies numbered from 1, **with** the cross-reference table §12.7.8.1
    /// makes optional.
    ///
    /// The table is here rather than in [`fdf`] because an embedded FDF is a stream whose bytes
    /// are themselves a file: they contain `obj` and `endobj`, so a document recovered by
    /// scanning would find the inner file's objects inside the outer file's stream. A real file
    /// carrying one states a table for the same reason.
    fn fdf_objects(bodies: &[String]) -> Document {
        Document::open(fdf_bytes(bodies).into_bytes())
            .expect("an FDF file is opened by the PDF reader")
    }

    /// The bytes [`fdf_objects`] opens, which an embedded file stream carries verbatim.
    fn fdf_bytes(bodies: &[String]) -> String {
        use std::fmt::Write as _;

        let mut out = String::from("%FDF-1.2\n");
        let mut offsets = Vec::new();
        for (index, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let section_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            bodies.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{section_at}\n%%EOF\n",
            bodies.len().saturating_add(1)
        );
        out
    }

    /// One whole FDF file, as the body of the embedded file stream that carries it.
    ///
    /// §7.11.4's stream is written uncompressed with a direct `/Length`, which §12.7.8.1 requires
    /// an FDF to do everywhere: "the length of a stream shall be specified by a direct object".
    fn embedded_body(file: &str) -> String {
        format!(
            "<< /Type /EmbeddedFile /Length {} >>\nstream\n{file}\nendstream",
            file.len()
        )
    }

    /// Table 246's `/EmbeddedFDFs` is "[a]n array of file specifications … representing other FDF
    /// files embedded within this one", with no deprecation marker on the cell — so each element
    /// is §7.11.4's embedded file stream carrying an FDF this reader reads like any other.
    ///
    /// Two of them, because one would not show the order: [`FormsData::files`] is the outer file
    /// and then the array's elements in the array's own order, which is what
    /// [`match_to_document`] applies "in turn".
    #[test]
    fn two_embedded_fdfs_are_read_as_the_files_they_are() {
        let document = fdf_objects(&[
            "<< /FDF << /Fields [ << /T (outer) /V (from the outer file) >> ]\n\
                /EmbeddedFDFs [ << /Type /Filespec /F (first.fdf) /EF << /F 2 0 R >> >>\n\
                                << /Type /Filespec /F (second.fdf) /EF << /F 3 0 R >> >> ] >>\n>>"
                .to_owned(),
            embedded_body(&fdf_bytes(&[
                "<< /FDF << /Fields [ << /T (inner) /V (first) >> ] /Status (one) >>\n>>"
                    .to_owned(),
            ])),
            embedded_body(&fdf_bytes(&[
                "<< /FDF << /Fields [ << /T (inner) /V (second) >> ] >>\n>>".to_owned(),
            ])),
        ]);
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.embedded.len(), 2, "both elements of the array");
        assert_eq!(
            data.files().len(),
            3,
            "this file and the two inside it, outermost first"
        );
        assert_eq!(
            data.embedded[0]
                .fields
                .first()
                .and_then(|field| field.value.as_ref())
                .and_then(Object::as_string),
            Some(b"first".as_slice())
        );
        assert_eq!(
            data.statuses(),
            ["one"],
            "Table 246 makes a /Status one that \"shall be displayed\", in whichever file states it"
        );
        assert!(
            data.owed.is_empty(),
            "nothing about an ordinary embedded FDF goes unapplied: {:?}",
            data.owed
        );

        // The order the pairs come back in is the order an import applies them, so the *second*
        // embedded file is the later statement about a widget both of them name.
        let widgets = std::collections::BTreeMap::from([
            ("outer".to_owned(), vec![ObjectId::new(10, 0)]),
            ("inner".to_owned(), vec![ObjectId::new(11, 0)]),
        ]);
        let (matched, unmatched) = match_to_document(&data, &widgets);
        assert!(unmatched.is_empty(), "{unmatched:?}");
        let values: Vec<_> = matched
            .iter()
            .map(|(id, import)| {
                (
                    id.number,
                    import
                        .value
                        .as_ref()
                        .and_then(Object::as_string)
                        .map(<[u8]>::to_vec),
                )
            })
            .collect();
        assert_eq!(
            values,
            [
                (10, Some(b"from the outer file".to_vec())),
                (11, Some(b"first".to_vec())),
                (11, Some(b"second".to_vec())),
            ]
        );
    }

    /// Table 247's `/EncryptionRevision` is "(Required if the FDF file is encrypted; deprecated in
    /// PDF 2.0)", and revision 1's 40-bit RC4 key is derived from a password this program has not
    /// got — so such a file is refused **by name** rather than read as plaintext (trap 5).
    #[test]
    fn an_encrypted_embedded_fdf_is_refused_by_name() {
        let document = fdf_objects(&[
            "<< /FDF << /EmbeddedFDFs [ << /Type /Filespec /F (secret.fdf) \
                /EF << /F 2 0 R >> >> ] >>\n>>"
                .to_owned(),
            "<< /Type /EmbeddedFile /EncryptionRevision 1 /Length 13 >>\nstream\nnot plaintext\nendstream"
                .to_owned(),
        ]);
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert!(data.embedded.is_empty());
        assert_eq!(
            data.owed,
            ["/EmbeddedFDFs: an encrypted FDF, whose Table 247 key derivation is deprecated"]
        );
    }

    /// A specification naming a file *outside* this one states no bytes to read, and an `/EF`
    /// with no stream states none either. Both are named rather than dropped.
    #[test]
    fn an_embedded_fdf_this_reader_cannot_reach_is_named() {
        let document = fdf("1 0 obj\n<< /FDF << /EmbeddedFDFs [ (elsewhere.fdf) \
             << /Type /Filespec /F (outside.fdf) >> ] >>\n>>\nendobj");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert!(data.embedded.is_empty());
        assert_eq!(
            data.owed,
            ["/EmbeddedFDFs: a file specification naming a file outside this one"],
            "the string form of §7.11.1's specification carries no /EF either, and is not a \
             second sentence"
        );
    }

    /// `CLAUDE.md` principle 3: the bound is on the nesting as a whole, and reaching it is
    /// reported rather than fatal.
    #[test]
    fn the_nesting_is_bounded_and_says_so() {
        // Each level embeds the one below it, so the nesting is built inside out.
        let mut inner =
            fdf_bytes(&["<< /FDF << /Fields [ << /T (deepest) /V (bottom) >> ] >>\n>>".to_owned()]);
        for _ in 0..=MAX_EMBEDDED_DEPTH {
            inner = fdf_bytes(&[
                "<< /FDF << /EmbeddedFDFs [ << /Type /Filespec /EF << /F 2 0 R >> >> ] >>\n>>"
                    .to_owned(),
                embedded_body(&inner),
            ]);
        }
        let document = Document::open(inner.into_bytes()).expect("an FDF file");
        let data = FormsData::read(&document).expect("an FDF catalog");
        let deep = data.files().len();
        assert_eq!(
            deep,
            MAX_EMBEDDED_DEPTH + 1,
            "the outermost file and one per level followed"
        );
        assert!(
            data.owed
                .contains(&"/EmbeddedFDFs: a nesting deeper than this reader follows"),
            "{:?}",
            data.owed
        );
    }

    /// Table 249's `/IF` names the same Table 250 dictionary Table 192's `/IF` does, so importing
    /// one replaces the widget's under §12.7.8.3.2's replacing sentence.
    #[test]
    fn an_imported_icon_fit_is_carried_to_the_widget() {
        let document = fdf(
            "1 0 obj\n<< /FDF << /Fields [ << /T (logo) /IF << /SW /N /S /A >> >> ] >>\n>>\nendobj",
        );
        let data = FormsData::read(&document).expect("an FDF catalog");
        let fit = data.fields[0]
            .icon_fit
            .as_ref()
            .expect("Table 249's /IF is read");
        assert_eq!(
            fit.get("SW")
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec()),
            Some(b"N".to_vec())
        );
        assert!(
            !data.fields[0].owed.iter().any(|why| why.starts_with("/IF")),
            "the entry is applied, so it is not owed: {:?}",
            data.fields[0].owed
        );

        let widgets =
            std::collections::BTreeMap::from([("logo".to_owned(), vec![ObjectId::new(7, 0)])]);
        let (matched, _) = match_to_document(&data, &widgets);
        assert_eq!(
            matched[0]
                .1
                .icon_fit
                .as_ref()
                .and_then(|fit| fit.get("S"))
                .and_then(Object::as_name)
                .map(|name| name.as_bytes().to_vec()),
            Some(b"A".to_vec())
        );
    }

    /// Table 245's `/Version` is ranked against the header by one rule, and that rule is
    /// [`pdf_syntax::Document::version`] — so an FDF written as §7.5.6's chain of updates reports
    /// the version the *last* revision reached rather than the newest catalog's entry (ADR 1171).
    #[test]
    fn a_multiply_updated_file_conforms_to_the_version_the_chain_reached() {
        use std::fmt::Write as _;

        let mut out = String::from("%FDF-1.2\n");
        let mut previous: Option<usize> = None;
        // Two revisions: the first states /Version 1.6, the second rewrites the catalog and
        // leaves the entry out. Reading the newest catalog alone would answer 1.2.
        for catalog in ["/Version /1.6 ", ""] {
            let catalog_at = out.len();
            let _ = write!(out, "1 0 obj\n<< /FDF << >> {catalog}>>\nendobj\n");
            let section_at = out.len();
            let _ = write!(
                out,
                "xref\n1 1\n{catalog_at:010} 00000 n \ntrailer\n<< /Root 1 0 R /Size 2"
            );
            if let Some(previous) = previous {
                let _ = write!(out, " /Prev {previous}");
            }
            let _ = write!(out, " >>\nstartxref\n{section_at}\n%%EOF\n");
            previous = Some(section_at);
        }
        let document = Document::open(out.into_bytes()).expect("an FDF file");
        let data = FormsData::read(&document).expect("an FDF catalog");
        assert_eq!(data.version, None, "the newest catalog states no entry");
        assert_eq!(
            data.conforms_to,
            Some(pdf_syntax::Version { major: 1, minor: 6 }),
            "the version a revision reached is not reduced by a later revision's silence"
        );
    }
}
