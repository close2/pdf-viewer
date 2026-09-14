//! ISO 32000-2 §12.7.6.2's submission: the names and values, composed here and transmitted by
//! whoever has a network.
//!
//! > Upon invocation of a submit-form action, an interactive PDF processor shall transmit the
//! > names and values of selected interactive form fields to a specified uniform resource
//! > locator (URL).
//!
//! That sentence has two halves and this crate can do one of them. *Which* names and values,
//! in *which* format, to *which* URL by *which* method are all functions of the document, of
//! Tables 239 and 240, and of what a person has entered since the file opened — every one of
//! which is state this crate holds. *Transmitting* is a network request, and a crate that runs
//! under principle 3's sandbox has no network and is not going to be given one. So [`compose`]
//! produces the request whole — URL, method, media type, body — and hands it up as a value; a
//! host is what sends it, under a policy the host supplies, or declines it and says so. The
//! division is ADR 1062's, and it is the same one §12.6.4.8's URI and §12.7.6.4's import already
//! sit on: the core decides what the document asked for, the host decides whether this machine
//! does it.
//!
//! # The four formats, and which are composed
//!
//! §12.7.6.2 lists four — "HTML Form format", "Forms Data Format (FDF)", "XFDF, a version of FDF
//! based on XML as defined by ISO 19444-1", and "PDF (in this case, the entire document shall be
//! submitted rather than individual fields and values)". Three are composed:
//!
//! - **FDF** is §12.7.8's own structure and this crate already reads it; [`fdf`] writes the same
//!   thing — a `%FDF-` header, one catalog object, a trailer naming it — with the fields as
//!   Table 249's dictionaries, nested down `/Kids` exactly as §12.7.4.2's qualified names nest.
//! - **HTML Form format** is "described in the HTML 4.01 Specification", whose section 17.13.4
//!   defines both of the content types a form is submitted with: section 17.13.4.1's
//!   `application/x-www-form-urlencoded`, which [`urlencoded`] is, and section 17.13.4.2's
//!   `multipart/form-data`, which [`multipart`] is. The second is §12.7.5.3's, required of any
//!   submission holding a file-select control, and it is why [`Submission::media_type`] is a
//!   string rather than [`Format::content_type`]'s constant. That specification is not a document
//!   this tree holds a copy of; it is a free W3C Recommendation at
//!   <https://www.w3.org/TR/html401/interact/forms.html>, and the quotations below were taken
//!   from it rather than from any reader's behaviour.
//! - **PDF** is the document with §7.5.6's update appended, which is what
//!   [`crate::view::ViewState::save`] already produces.
//!
//! **XFDF is declined by name**, for the reason §12.7.6.4's import declines it: ISO 19444-1 is
//! not on this disk, and `CLAUDE.md` principle 5 makes a grammar taken from another reader or from
//! sample files not a reading of a specification at all.
//!
//! # What is composed and what is owed, by flag
//!
//! Every flag of Table 240 is read, and each is either applied or named on
//! [`Submission::owed`], because a submission that silently dropped a flag the document set
//! would be trap 5's silence inside a feature otherwise built:
//!
//! | bit | name | here |
//! |---|---|---|
//! | 1 | `Include/Exclude` | applied, with Table 239's `/Fields` |
//! | 2 | `IncludeNoValueFields` | applied |
//! | 3 | `ExportFormat` | applied: HTML Form format against FDF, and §12.7.5.3 decides which of that format's two content types |
//! | 4 | `GetMethod` | applied; owed where set against a clear bit 3, which the table forbids, and where §12.7.5.3's body leaves a GET nowhere to put it |
//! | 5 | `SubmitCoordinates` | applied from the click, where there was one |
//! | 6 | `XFDF` | declined: ISO 19444-1 is not held |
//! | 7 | `IncludeAppendSaves` | applied: Table 246's `/Differences` holds what the update appended |
//! | 8 | `IncludeAnnotations` | applied: §12.7.8.3.4's annotation dictionaries, with Table 254's `/Page` |
//! | 9 | `SubmitPDF` | applied |
//! | 10 | `CanonicalFormat` | owed: which fields hold dates "is not specified explicitly in the field itself but only in the ECMAScript code that processes it" (its NOTE 1), and ECMAScript is excluded |
//! | 11 | `ExclNonUserAnnots` | owed: the name it narrows by is the *server*'s, so bit 8's annotations are withheld whole rather than sent past the narrowing |
//! | 12 | `ExclFKey` | applied: it is what bit 14's `/F` is written against |
//! | 14 | `EmbedForm` | applied: §7.11.4's embedded file stream, minus the path Table 43 asks for and this crate has none of |
//!
//! **Three of those were `owed` until the one-thousand-and-fifty-second session, and the reason
//! they moved is that none of them was ever about a network.** Bit 7's `/Differences` is
//! §7.5.6's update, which [`crate::view::ViewState::save`] already writes; bit 8's annotations
//! are the document's own dictionaries; bit 14's embedded file is that same save. Bit 11 is the
//! one that stayed, and it stayed for the standard's own reason rather than for want of code —
//! see [`Carried`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId, Stream};

use crate::action::{ResetTarget, SubmitForm};
use crate::appearance::{FLAG_FILE_SELECT, FLAG_NO_EXPORT, Field, FieldKind};
use crate::view::{ViewState, widgets_by_field_name, widgets_under};

/// The request a submit-form action composes, ready for a host to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    /// Where to: Table 239's `/F`, with the form data appended as a query for an HTTP GET.
    pub url: String,
    /// How: Table 240 bit 4.
    pub method: Method,
    /// What the body is.
    pub format: Format,
    /// The media type a host puts on the request, which is the one to send.
    ///
    /// [`Format::content_type`] is the *format's* type and this is the *submission's*, and
    /// §12.7.5.3 is why the two are not always one string: a form holding a file-select control
    /// "shall use the MIME content type multipart / form-data", which carries RFC 2046 section
    /// 5.1.1's required `boundary` parameter and so cannot be a constant. Everywhere else this
    /// is exactly `format.content_type()`.
    pub media_type: String,
    /// The body. Empty for a GET, whose data is in [`Self::url`].
    pub body: Vec<u8>,
    /// How many fields the body names.
    pub fields: usize,
    /// What the action asked for that this composition does not do, one sentence each.
    ///
    /// A host prints these beside the request, because a person who pressed a button is owed
    /// the difference between what the document asked for and what was made.
    pub owed: Vec<String>,
}

/// Table 240 bit 4: "If set, field names and values shall be submitted using an HTTP GET request.
/// If clear, they shall be submitted using a POST request."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// An HTTP GET: the data is in the URL's query and the body is empty.
    Get,
    /// An HTTP POST: the data is the body.
    Post,
}

/// Which of §12.7.6.2's formats the body is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// "Forms Data Format (FDF), which is described in 12.7.8".
    Fdf,
    /// "HTML Form format (described in the HTML 4.01 Specification)".
    HtmlForm,
    /// "PDF (in this case, the entire document shall be submitted rather than individual fields
    /// and values)".
    Pdf,
}

impl Format {
    /// The media type of a body written in this format alone.
    ///
    /// [`Submission::media_type`] is what a host actually sends, and the two differ for one
    /// reason: §12.7.5.3 makes a form holding a file-select control `multipart/form-data`, whose
    /// RFC 2046 section 5.1.1 `boundary` parameter is a function of the body rather than of the
    /// format.
    ///
    /// Table 240 bit 9 states the third — "the MIME media type application/pdf as defined by
    /// Internet RFC 8118" — and HTML 4.01 section 17.13.4 states the second. ISO 32000-2 states
    /// none for FDF, so the third comes from the registry rather than from a habit:
    /// `application/fdf`, registered 2022-04-05 by ISO TC 171/SC 2 — the committee that owns this
    /// standard — at <https://www.iana.org/assignments/media-types/application/fdf>. The older
    /// `application/vnd.fdf` is a vendor-tree name for the same bytes and is not what the
    /// registry lists.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Fdf => "application/fdf",
            Self::HtmlForm => "application/x-www-form-urlencoded",
            Self::Pdf => "application/pdf",
        }
    }
}

/// Why a submission could not be composed at all.
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    /// Table 240 bit 6.
    #[error(
        "SubmitForm: Table 240 bit 6 asks for XFDF, whose ISO 19444-1 is not held, so no reading \
         of it can be written"
    )]
    Xfdf,
    /// Table 240 bit 9 asks for the document, and §7.5.6's update cannot be appended to it.
    #[error(
        "SubmitForm: Table 240 bit 9 asks for the whole document, which cannot be written: {0}"
    )]
    Unsavable(#[from] pdf_syntax::write::UpdateError),
    /// §12.7.5.3's multipart body has no delimiter its own parts do not contain.
    ///
    /// HTML 4.01 section 17.13.4.2 requires that "[p]art boundaries should not occur in any of
    /// the data" and says outright that "how this is done lies outside the scope of this
    /// specification". [`MAX_BOUNDARY_TRIES`] is how far this composition looks; a body that
    /// contains every delimiter it would try is one no server could split back into parts, and
    /// writing it anyway would hand over a request that silently means something else.
    #[error(
        "SubmitForm: §12.7.5.3's multipart/form-data body contains every delimiter this \
         composition can form, so no part boundary would separate it"
    )]
    Undelimitable,
}

/// The click that invoked the action, for Table 240 bit 5.
///
/// Two things and not three: the cursor in default user space, and the annotation it was over.
/// The rectangle the bit measures from is read here rather than passed in, because the bit
/// names a *particular* one — "the field's widget annotation rectangle" — where §12.6.4.8's
/// `/IsMap` names "the annotation with which the URI action is associated". The two are the
/// same annotation whenever a widget was pressed and different when a `/Link` carries the
/// action, and a caller handing over one rectangle for both would be answering the second
/// clause's question under the first clause's name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    /// The cursor, in default user space.
    pub point: (f32, f32),
    /// The annotation that was pressed, where the caller knows which.
    ///
    /// `None` for an action raised with no annotation under the pointer — an outline item, a
    /// navigation node — which Table 240 bit 5 has no rectangle to measure from and which
    /// [`Submission::owed`] therefore names.
    pub widget: Option<ObjectId>,
}

/// One field as it goes into the body.
struct Entry {
    /// §12.7.4.2's fully qualified name.
    name: String,
    /// Table 226's `/V`, resolved, or `None` for a field submitted "by name only".
    value: Option<Object>,
    /// The file this entry carries, where §12.7.5.3's `FileSelect` makes it a file-select
    /// control and this document holds the file it names.
    file: Option<SelectedFile>,
}

/// §12.7.5.3's file-select control, with the file the field's value names.
///
/// Table 231 bit 21 says "the text entered in the field represents the pathname of a file whose
/// contents shall be submitted as the value of the field", and the clause under it says what
/// *identifies* that file: "a file specification (7.11, "File specifications") identifying the
/// selected file". §7.11.1 gives a specification two forms, and only one of them carries the file
/// along — §7.11.4's embedded file stream, reached through Table 43's `/EF`. That is the one this
/// crate can read, and it is the only one: a specification in the string form names a path on a
/// filesystem `CLAUDE.md` principle 3 gives this process none of, and [`Submission::owed`] says
/// so by field name rather than submitting the pathname as though it were the contents.
struct SelectedFile {
    /// The specification's own name, for HTML 4.01 section 17.13.4.2's `filename` parameter —
    /// "[t]he user agent should attempt to supply a file name for each submitted file".
    name: Option<String>,
    /// Table 44's `/Subtype`, the embedded file stream's own media type.
    ///
    /// HTML 4.01 section 17.13.4.2 asks for one and names the fallback: "the file input should be
    /// identified by the appropriate content type (e.g., "application/octet-stream")".
    media_type: String,
    /// §7.11.4's contents, decoded.
    bytes: Vec<u8>,
}

/// Composes §12.7.6.2's submission for one action over one document as this view shows it.
///
/// The clause's selection rules, in the order they bind:
///
/// > The NoExport flag in the field dictionary's Ff entry … takes precedence over the action's
/// > Fields array and Include/ Exclude flag. Fields whose NoExport flag is set shall not be
/// > included in a submit-form action.
///
/// > If the Include/Exclude flag is clear, the submission consists of all fields listed in the
/// > Fields array, along with any descendants of those fields in the field hierarchy. If the
/// > Include/Exclude flag is set, the submission shall consist of all fields in the document's
/// > interactive form except those listed in the Fields array.
///
/// > Fields with no value (that is, whose field dictionary does not contain a V entry) are
/// > ordinarily not included in the submission. The submitform action's IncludeNoValueFields flag
/// > may override this behaviour. If this flag is set, such valueless fields shall be included in
/// > the submission by name only, with no associated value.
///
/// > For push-button fields submitted in FDF, the value submitted shall be that of the AP entry in
/// > the field's widget annotation dictionary. If the submit-form action dictionary contains no
/// > Fields entry, such pushbutton fields shall not be submitted.
///
/// A push-button *named* by `/Fields` is owed rather than submitted: its value would be its
/// appearance streams, which live in this document's object space, and carrying them into an FDF
/// is the second-document question §12.7.8.3.2's row leaves open. The value of every other field
/// is "specified by the V entry" — whichever of the four statements about it this view holds
/// current, which is why this takes a [`ViewState`] and not a document alone.
///
/// # Errors
///
/// [`Refusal`]: XFDF, or a document the update cannot be appended to.
pub fn compose(
    document: &Document,
    view: &ViewState,
    action: &SubmitForm,
    click: Option<&Click>,
) -> Result<Submission, Refusal> {
    let flags = action.flags;
    let mut owed = Vec::new();

    // Table 240 bit 9: "If set, the document shall be submitted as PDF … If set, all other
    // flags shall be ignored except GetMethod." Read before bit 6 for that reason.
    if flags.submit_pdf() {
        return whole_document(document, view, action, owed);
    }
    // Table 240 bit 6: "shall be used only if the SubmitPDF flags are clear. If set, field
    // names and values shall be submitted as XFDF."
    if flags.xfdf() {
        return Err(Refusal::Xfdf);
    }
    // Table 240 bit 3: "If set, field names and values shall be submitted in HTML Form format.
    // If clear, they shall be submitted in Forms Data Format (FDF)." Two formats and not three,
    // because bit 9's PDF returned above — which is why this is one boolean carried down rather
    // than a match over [`Format`] with an arm that cannot happen.
    let html = flags.export_format();
    let format = if html { Format::HtmlForm } else { Format::Fdf };

    let table = widgets_by_field_name(document);
    let chosen = chosen(document, view, action, format, &table, &mut owed);
    let entries = chosen.entries;
    let fields = entries.len();
    let (url, method, body, media_type) = if html {
        if let Some(charset) = action
            .charset
            .as_deref()
            .filter(|charset| !charset.eq_ignore_ascii_case("utf-8"))
        {
            owed.push(format!(
                "Table 239's /CharSet {charset} is not applied; names and values are written \
                 in UTF-8"
            ));
        }
        if flags.canonical_format() {
            owed.push(
                "Table 240 bit 10's CanonicalFormat is not applied: which fields hold dates is \
                 stated only by ECMAScript, which CLAUDE.md principle 5 excludes"
                    .to_owned(),
            );
        }
        let placed = flags
            .submit_coordinates()
            .then(|| coordinates(document, &table, click, &mut owed))
            .flatten();
        // §12.7.5.3: "For fields submitted in HTML Form format, the submission shall use the
        // MIME content type multipart / form-data, as described in Internet RFC 2045." The
        // condition is a file-select control being in the submission, not its file being
        // readable, so this is `Chosen::file_select` rather than a look at the entries.
        if chosen.file_select {
            // Two `shall`s the document has asked for at once, and they cannot both be met: an
            // HTTP GET carries its data in the URL and has no entity body for a media type to
            // describe. The specific one wins — bit 4 speaks about "field names and values" in
            // general and §12.7.5.3 about the control whose file is the reason there is a body
            // — and the general one goes to the host as a sentence.
            if flags.get_method() {
                owed.push(
                    "Table 240 bit 4's GetMethod asks for an HTTP GET, whose data is a URL \
                     query with no body; §12.7.5.3's file-select control requires a \
                     multipart/form-data body, so this is composed as a POST"
                        .to_owned(),
                );
            }
            let (boundary, body) = multipart(document, &entries, placed.as_ref(), &mut owed)?;
            (
                action.url.clone(),
                Method::Post,
                body,
                format!("multipart/form-data; boundary={boundary}"),
            )
        } else {
            let mut query = urlencoded(document, &entries, &mut owed);
            if let Some(placed) = &placed {
                if !query.is_empty() {
                    query.push('&');
                }
                // The name is escaped and the full stop the table puts after it is not: the
                // PERIOD is the format's punctuation rather than a character of the name.
                let mut prefix = String::new();
                if let Some(named) = &placed.named {
                    escape(named.as_bytes(), &mut prefix);
                    prefix.push('.');
                }
                let (across, down) = (placed.across, placed.down);
                let _ = write!(query, "{prefix}x={across}&{prefix}y={down}");
            }
            if flags.get_method() {
                (
                    with_query(&action.url, &query),
                    Method::Get,
                    Vec::new(),
                    format.content_type().to_owned(),
                )
            } else {
                (
                    action.url.clone(),
                    Method::Post,
                    query.into_bytes(),
                    format.content_type().to_owned(),
                )
            }
        }
    } else {
        let carried = Carried::read(document, view, action, &mut owed);
        (
            action.url.clone(),
            Method::Post,
            fdf(document, &entries, &carried, &mut owed),
            format.content_type().to_owned(),
        )
    };
    Ok(Submission {
        url,
        method,
        format,
        media_type,
        body,
        fields,
        owed,
    })
}

/// Table 240 bit 9's whole document, which "shall be submitted rather than individual fields and
/// values".
///
/// §7.5.6's update over the file as it stands, which is `ViewState::save`'s — the same bytes a
/// person saving the document would get, and the reason the bit says every other flag is ignored:
/// there are no field names in this body to select between.
///
/// # Errors
///
/// [`Refusal::Unsavable`], where the update cannot be appended.
fn whole_document(
    document: &Document,
    view: &ViewState,
    action: &SubmitForm,
    mut owed: Vec<String>,
) -> Result<Submission, Refusal> {
    let written = view.save(document)?;
    // §12.7.5.3's Table 231 bit 14 keeps a password out of the file this writes, so the
    // submission carries less than the form holds; a person is owed that sentence.
    for name in written.withheld {
        owed.push(format!(
            "field {name}: a Table 231 bit 14 password, whose value is not written into the \
             document submitted"
        ));
    }
    Ok(Submission {
        url: action.url.clone(),
        method: if action.flags.get_method() {
            Method::Get
        } else {
            Method::Post
        },
        format: Format::Pdf,
        media_type: Format::Pdf.content_type().to_owned(),
        body: written.bytes,
        fields: 0,
        owed,
    })
}

/// What [`chosen`] read out of the form: the entries, and whether a file-select control is
/// among them.
///
/// The second is not derivable from the first. §12.7.5.3's media-type sentence is about the
/// *submission*, so a file-select field whose file this document does not carry still decides
/// what the body is written as while contributing no entry to it.
#[derive(Default)]
struct Chosen {
    /// The fields that go into the body, in the order [`selected_names`] gives them.
    entries: Vec<Entry>,
    /// Whether any selected field sets Table 231 bit 21.
    file_select: bool,
}

/// The fields that go into the body, with the values this view shows for them.
///
/// [`selected_names`] applies Table 239's array and Table 240 bit 1; this applies everything the
/// clause says about a field rather than about the action — `NoExport`, the push-button sentence,
/// the valueless-field sentence, and §12.7.5.3's file-select control.
fn chosen(
    document: &Document,
    view: &ViewState,
    action: &SubmitForm,
    format: Format,
    table: &BTreeMap<String, Vec<ObjectId>>,
    owed: &mut Vec<String>,
) -> Chosen {
    let flags = action.flags;
    let selected = selected_names(document, action, table);
    let mut chosen = Chosen::default();

    for name in &selected {
        let Some(widget) = table.get(name).and_then(|widgets| widgets.first()) else {
            continue;
        };
        let object = document.get(*widget);
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let field = Field::read(document, dict, view.annotation(*widget).value);
        if field.too_deep {
            owed.push(format!(
                "field {name}: its /Parent chain is deeper than this reader walks, so \
                 §12.7.4.1's inheritance could not be resolved and neither its flags nor its \
                 value are known"
            ));
            continue;
        }
        if field.flags & FLAG_NO_EXPORT != 0 {
            continue;
        }
        if matches!(field.kind, Some(FieldKind::Button { toggling: false })) {
            if !action.fields.is_empty() && format == Format::Fdf {
                owed.push(format!(
                    "push-button {name}: its submitted value would be its /AP appearance \
                     streams, which are this document's objects and are not carried into an FDF"
                ));
            }
            continue;
        }
        // §12.7.5.3, under Table 231 bit 21: "If the FileSelect flag (PDF 1.4) is set, the field
        // shall function as a file-select control. In this case, the field's text represents the
        // pathname of a file whose contents shall be submitted as the field's value".
        //
        // Read here rather than beside the value, because the flag decides the HTML-form body's
        // media type whether or not there is a value behind it: the sentence that states the
        // media type is about the *submission*, and its condition is the flag.
        let file_select =
            matches!(field.kind, Some(FieldKind::Text)) && field.flags & FLAG_FILE_SELECT != 0;
        chosen.file_select |= file_select;
        let value = field
            .value
            .as_ref()
            .map(|value| resolved(document, name, value, owed));
        // A stream that would not decode is a field whose value this reader does not know, which
        // is not the same as a field that has none: it is left out even where bit 2 is set.
        if matches!(value, Some(None)) {
            continue;
        }
        let value = value.flatten();
        // A cleared field — `FieldValue::Edited` with no value — and a field stating no `/V`
        // are the same thing to this clause: "whose field dictionary does not contain a V
        // entry".
        let Some(value) = value else {
            if flags.include_no_value_fields() {
                chosen.entries.push(Entry {
                    name: name.clone(),
                    value: None,
                    file: None,
                });
            }
            continue;
        };
        if file_select {
            let selected = selected_file(document, name, &value, owed);
            match format {
                // "For Forms Data Format (FDF) submission, the value of the V entry in the FDF
                // field dictionary … shall be a file specification (7.11, "File specifications")
                // identifying the selected file."
                Format::Fdf => match selected {
                    // §7.11.1's dictionary form carrying §7.11.4's stream: an FDF has a body to
                    // put an indirect stream in, so the contents travel rather than the name of
                    // a file the server has no copy of.
                    Selected::Carried(file) => chosen.entries.push(Entry {
                        name: name.clone(),
                        value: None,
                        file: Some(file),
                    }),
                    // §7.11.1's string form, "just the name of the target file in a standard
                    // format". The pathname a person typed is written as typed: §7.11.2's
                    // platform-independent spelling of a path is a host's knowledge of its own
                    // filesystem, which this crate has none of. A choice, recorded.
                    Selected::Elsewhere(pathname) => chosen.entries.push(Entry {
                        name: name.clone(),
                        value: Some(Object::String(pathname.into())),
                        file: None,
                    }),
                    Selected::Nothing => {}
                },
                // "For fields submitted in HTML Form format, the submission shall use the MIME
                // content type multipart / form-data, as described in Internet RFC 2045" — whose
                // part carries the file's *contents*, which is what `multipart` writes.
                Format::HtmlForm => match selected {
                    Selected::Carried(file) => chosen.entries.push(Entry {
                        name: name.clone(),
                        value: None,
                        file: Some(file),
                    }),
                    // A pathname and no file. Submitting the pathname as the value would be the
                    // wrong value under the right name — the clause asks for the contents — so
                    // the field is named rather than guessed at.
                    Selected::Elsewhere(pathname) => owed.push(format!(
                        "field {name}: a file-select control naming {}, a file outside this \
                         document, whose contents multipart/form-data would carry and this \
                         process has no filesystem to read",
                        pdf_syntax::text_string(&pathname)
                    )),
                    Selected::Nothing => {}
                },
                Format::Pdf => {}
            }
            continue;
        }
        chosen.entries.push(Entry {
            name: name.clone(),
            value: Some(value),
            file: None,
        });
    }

    chosen
}

/// What a file-select control's `/V` turned out to name.
///
/// §7.11.1 gives a file specification two forms, and this is the difference between them stated
/// where it decides what a submission can carry: the dictionary form may hold the file, and the
/// string form can only name it.
enum Selected {
    /// §7.11.4's embedded file stream the specification carries — the contents themselves.
    Carried(SelectedFile),
    /// A specification naming a file outside this document, with the bytes that name it.
    Elsewhere(Vec<u8>),
    /// No file specification, or one whose stream this reader could not decode. Whoever
    /// produced this has already put the sentence on [`Submission::owed`].
    Nothing,
}

/// Reads a file-select control's value as §7.11's file specification.
///
/// > A simple file specification shall give just the name of the target file in a standard format
/// > … It shall take the form of either a string or a dictionary.
///
/// Two forms and no third, which is why a `/V` that is a name or a number is [`Selected::Nothing`]
/// here although [`text_bytes`] would give characters for it: a name is not a file specification,
/// and submitting one as though it were would name a file the field never selected.
fn selected_file(
    document: &Document,
    name: &str,
    value: &Object,
    owed: &mut Vec<String>,
) -> Selected {
    match document.resolve(value) {
        Object::String(bytes) => Selected::Elsewhere(bytes.to_vec()),
        Object::Dictionary(dict) => {
            let specification = crate::file_spec::FileSpec::from_dictionary(document, &dict);
            let Some(attachment) = crate::attachment::read(document, &dict, String::new()) else {
                if let Some(bytes) = specification.bytes {
                    return Selected::Elsewhere(bytes);
                }
                // Table 43 makes `/F` "[r]equired if the DOS, Mac, and Unix entries are all
                // absent", so a dictionary with none of them and no `/EF` identifies nothing.
                owed.push(format!(
                    "field {name}: a file-select control whose value is a file specification \
                     naming no file at all, so nothing is submitted for it"
                ));
                return Selected::Nothing;
            };
            match document.decoded_stream_data_reported(&attachment.stream) {
                Ok(decoded) => Selected::Carried(SelectedFile {
                    name: specification.display_name(),
                    // Table 44's `/Subtype` is the stream's own media type where it states one.
                    media_type: attachment
                        .media_type
                        .unwrap_or_else(|| "application/octet-stream".to_owned()),
                    bytes: decoded.data.to_vec(),
                }),
                // Trap 5: an embedded file that would not decode is a file whose contents this
                // reader does not know, which is not the same as a file of no bytes.
                Err(refusal) => {
                    owed.push(format!(
                        "field {name}: a file-select control whose embedded file stream this \
                         reader could not decode ({refusal:?}), so the field is not submitted at \
                         all rather than submitted empty"
                    ));
                    Selected::Nothing
                }
            }
        }
        other => {
            owed.push(format!(
                "field {name}: a file-select control whose value is {}, which §7.11.1 makes no \
                 file specification, so no file is submitted for it",
                other.type_name()
            ));
            Selected::Nothing
        }
    }
}

/// The fully qualified names the action selects, before `NoExport` and before values.
///
/// [`ResetTarget::Name`] is a prefix test, because a descendant's fully qualified name is its
/// ancestor's with `.` and more appended (§12.7.4.2); [`ResetTarget::Field`] is a walk down
/// `/Kids` to the widgets and back to their names, because a reference names a field that may
/// have several.
fn selected_names(
    document: &Document,
    action: &SubmitForm,
    table: &BTreeMap<String, Vec<ObjectId>>,
) -> Vec<String> {
    if action.fields.is_empty() {
        return table.keys().cloned().collect();
    }
    let by_widget: BTreeMap<ObjectId, &str> = table
        .iter()
        .flat_map(|(name, widgets)| widgets.iter().map(move |widget| (*widget, name.as_str())))
        .collect();
    let named: BTreeSet<&str> = action
        .fields
        .iter()
        .flat_map(|target| match target {
            ResetTarget::Field(id) => widgets_under(document, *id)
                .into_iter()
                .filter_map(|widget| by_widget.get(&widget).copied())
                .collect::<Vec<_>>(),
            ResetTarget::Name(name) => table
                .keys()
                .filter(|candidate| {
                    *candidate == name
                        || candidate
                            .strip_prefix(name.as_str())
                            .is_some_and(|rest| rest.starts_with('.'))
                })
                .map(String::as_str)
                .collect(),
        })
        .collect();
    table
        .keys()
        .filter(|name| named.contains(name.as_str()) != action.flags.exclude())
        .cloned()
        .collect()
}

/// A value with every reference followed, so that the body carries objects and not identities.
///
/// A stream is §12.7.5.3's "beginning with PDF 1.5, a stre am" form of a text value, and goes in
/// as the text it holds: an FDF has no object of this document's to refer to.
///
/// A stream this reader cannot decode — an unsupported filter, or one over the document's own
/// budget — is `None` and **not** an empty string. The two are different values and the second
/// one lies: it submits the field as though the person had left it blank, which is trap 5's
/// silence. Inside §12.7.5.4's array of selected values the same refusal drops that element and
/// keeps the others, because an array is a list of independent choices and the rest of them are
/// still what the person chose; either way the sentence goes on [`Submission::owed`], so a
/// shorter list is reported rather than merely shorter.
fn resolved(
    document: &Document,
    name: &str,
    value: &Object,
    owed: &mut Vec<String>,
) -> Option<Object> {
    Some(match document.resolve(value) {
        Object::Array(items) => Object::Array(
            items
                .iter()
                .filter_map(|item| resolved(document, name, item, owed))
                .collect(),
        ),
        Object::Stream(stream) => match document.decoded_stream_data_reported(&stream) {
            Ok(decoded) => Object::String(decoded.data.to_vec().into()),
            Err(refusal) => {
                owed.push(format!(
                    "field {name}: its value is a stream this reader could not decode \
                     ({refusal:?}), so the field is not submitted at all rather than submitted empty"
                ));
                return None;
            }
        },
        other => other,
    })
}

/// The characters of a resolved value, as UTF-8, for a body that carries text rather than
/// objects. `None` for a value that is not text and not a name.
fn text_bytes(document: &Document, value: &Object) -> Option<Vec<u8>> {
    match document.resolve(value) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes).into_bytes()),
        // A check box's or radio button's value is §12.7.5.2.3's appearance-state name, and
        // §7.3.5 makes a name's bytes the thing it is.
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        _ => None,
    }
}

/// Most markup annotations one submission carries, which is what this program's own reader takes.
///
/// [`crate::forms_data::MAX_ANNOTATIONS`] is where an FDF this program *reads* stops, so writing
/// past it would compose a file this program would not read back whole. Trap 38's question has no
/// other answer here: ISO 32000-2 states no bound on Table 246's `/Annots`.
const MAX_ANNOTATIONS: usize = crate::forms_data::MAX_ANNOTATIONS;

/// How deep an annotation's own entries are examined for a reference into this document.
///
/// Table 172 and Table 166 nest two levels at most — `/BS`, `/BE`, `/ExData`, an `/AP` with its
/// `/N` subdictionary — so a value deeper than this is a file doing something the tables do not
/// describe, and it is treated as unportable rather than walked further.
const MAX_ENTRY_DEPTH: usize = 8;

/// §12.5.6.2's markup annotations, which Table 171's third column names one at a time.
///
/// The eighteen types whose `Markup` column says `Yes`, and Table 246's `/Annots` note is
/// consistent with them: the six subtypes it excludes — "Link, Movie, Widget, `PrinterMark`,
/// Screen, and `TrapNet`" — say `No` in that column.
const MARKUP: [&str; 18] = [
    "Text",
    "FreeText",
    "Line",
    "Square",
    "Circle",
    "Polygon",
    "PolyLine",
    "Highlight",
    "Underline",
    "Squiggly",
    "StrikeOut",
    "Caret",
    "Stamp",
    "Ink",
    "FileAttachment",
    "Sound",
    "Redact",
    "Projection",
];

/// What Table 240's bits 7, 8, 11, 12 and 14 put into an FDF beside its fields.
///
/// Every one of the three this carries is a reading of the *document*, which is why none of them
/// needed the network the whole clause was refused for: bit 7's `/Differences` and bit 14's
/// embedded file are both [`ViewState::save`]'s §7.5.6 update, and bit 8's annotations are
/// dictionaries the file already states.
///
/// **Bit 11 is the one that is genuinely somebody else's**, and §12.7.6.2's Table 240 says
/// whose:
///
/// > If set, it shall include only those markup annotations whose T entry … matches the name of
/// > the current user, as determined by the remote server to which the form is being submitted.
///
/// The name is *the server's* determination, and this program has none of its own either —
/// [`ViewState::add_markup`] writes no `/T` for exactly that reason. So the predicate cannot be
/// evaluated here, and of the two ways to be wrong about it only one breaks a `shall`: sending an
/// annotation that does not match breaks bit 11's *only*, and sending none breaks nothing the
/// clause states, since bit 11 is itself the narrowing of bit 8. The narrowing is therefore
/// applied to the whole set, and the sentence saying so goes to the host that has the server.
#[derive(Default)]
struct Carried {
    /// Table 246's `/Differences`, for bit 7.
    differences: Option<Vec<u8>>,
    /// The file itself, for bit 14's embedded file stream.
    embedded: Option<Vec<u8>>,
    /// Table 246's `/Annots`, for bit 8.
    annotations: Vec<Object>,
}

impl Carried {
    /// Reads every Table 240 flag an FDF submission answers past its field list.
    ///
    /// Bits 4 and 5 are here too and carry nothing: each "shall be used only when the
    /// `ExportFormat` flag is set", which in this branch it is not, so what they get is a sentence.
    fn read(
        document: &Document,
        view: &ViewState,
        action: &SubmitForm,
        owed: &mut Vec<String>,
    ) -> Self {
        let flags = action.flags;
        let mut carried = Self::default();
        if flags.get_method() {
            owed.push(
                "Table 240 bit 4's GetMethod is set against a clear ExportFormat, which the table \
                 forbids (\"if ExportFormat is clear, this flag shall also be clear\"); the FDF \
                 is sent by POST"
                    .to_owned(),
            );
        }
        if flags.submit_coordinates() {
            owed.push(
                "Table 240 bit 5's SubmitCoordinates \"shall be used only when the ExportFormat \
                 flag is set\", and it is clear; no coordinates are written into the FDF"
                    .to_owned(),
            );
        }
        if flags.canonical_format() {
            owed.push(
                "Table 240 bit 10's CanonicalFormat is not applied: which fields hold dates is \
                 stated only by ECMAScript, which CLAUDE.md principle 5 excludes"
                    .to_owned(),
            );
        }
        carried.save(document, view, action, owed);
        carried.mark_up(document, view, action, owed);
        carried
    }

    /// Bits 7 and 14, which are one save between them.
    ///
    /// Table 246's `/Differences` row states the save as a requirement rather than as a
    /// convenience — "[a]n incremental update shall be automatically performed just before the
    /// submission takes place, in order to capture all changes made to the document" — and bit
    /// 14's embedded file is "the PDF file from which the FDF is being submitted", which after
    /// that sentence is the same bytes. So [`ViewState::save`] runs once and both read it.
    fn save(
        &mut self,
        document: &Document,
        view: &ViewState,
        action: &SubmitForm,
        owed: &mut Vec<String>,
    ) {
        let flags = action.flags;
        // Bit 14 against bit 12 asks for an entry the same table forbids, and the forbidding one
        // wins: an `/F` written past "the submitted FDF shall exclude the F entry" would be the
        // one flag of the two that this composition chose to disobey.
        let embed = flags.embed_form() && !flags.exclude_f_key();
        if flags.embed_form() && flags.exclude_f_key() {
            owed.push(
                "Table 240 bit 14's EmbedForm asks that \"the F entry of the submitted FDF shall \
                 be a file specification containing an embedded file stream\" and bit 12's \
                 ExclFKey says \"the submitted FDF shall exclude the F entry\"; the exclusion is \
                 applied and the document is not embedded"
                    .to_owned(),
            );
        }
        if !flags.include_append_saves() && !embed {
            return;
        }
        let written = match view.save(document) {
            Ok(written) => written,
            Err(why) => {
                owed.push(format!(
                    "Table 240 bits 7 and 14 need §7.5.6's update, which this document cannot be \
                     given: {why}"
                ));
                return;
            }
        };
        // §12.7.5.3's Table 231 bit 14 keeps a password out of what `save` writes, so an FDF
        // carrying that file carries less than the form holds — the same sentence
        // [`whole_document`] owes, for the same bytes.
        for name in &written.withheld {
            owed.push(format!(
                "field {name}: a Table 231 bit 14 password, whose value is not in the document \
                 bytes this FDF carries"
            ));
        }
        if flags.include_append_saves() {
            // "A stream containing all the bytes in all incremental updates made to the
            // underlying PDF document since it was opened" — so the file as it was opened is
            // where the stream starts, and everything `save` appended to it is the stream.
            let opened = document.bytes().len();
            match written.bytes.get(opened..) {
                Some(appended) if !appended.is_empty() => {
                    self.differences = Some(appended.to_vec());
                }
                _ => owed.push(
                    "Table 240 bit 7's IncludeAppendSaves: the save appended nothing to the file \
                     as it was opened, so there is no /Differences stream to write"
                        .to_owned(),
                ),
            }
        }
        if embed {
            // The gap [`embedded_file_spec`] argues, said out loud: the specification carries the
            // file and cannot name it.
            owed.push(
                "Table 240 bit 14's EmbedForm: the /F written carries the document as \
                 §7.11.4's embedded file stream and states no name for it, because Table 43's \
                 own /F is \"[a] file specification string of the form described in 7.11.2\" — a \
                 path on this machine, which this crate has none of"
                    .to_owned(),
            );
            self.embedded = Some(written.bytes);
        }
    }

    /// Bits 8 and 11, which decide between them whether any annotation goes.
    fn mark_up(
        &mut self,
        document: &Document,
        view: &ViewState,
        action: &SubmitForm,
        owed: &mut Vec<String>,
    ) {
        let flags = action.flags;
        match (
            flags.include_annotations(),
            flags.exclude_non_user_annotations(),
        ) {
            // Bit 8: "all markup annotations in the underlying PDF document".
            (true, false) => self.annotations = annotations(document, view, owed),
            (true, true) => owed.push(
                "Table 240 bit 11's ExclNonUserAnnots narrows bit 8's annotations to those whose \
                 /T \"matches the name of the current user, as determined by the remote server to \
                 which the form is being submitted\", and no part of this program knows that \
                 name; the narrowing is applied to all of them, so this FDF carries none"
                    .to_owned(),
            ),
            (false, true) => owed.push(
                "Table 240 bit 11's ExclNonUserAnnots shall be used only when \"the \
                 IncludeAnnotations flag is set\", and bit 8 is clear; no annotations are written"
                    .to_owned(),
            ),
            (false, false) => {}
        }
    }
}

/// Table 246's `/Annots` for Table 240 bit 8: every §12.5.6.2 markup annotation this document
/// holds, as §12.7.8.3.4 asks for them.
///
/// > Each annotation dictionary in an FDF file shall have a Page entry … that shall indicate the
/// > page of the source document to which the annotation is attached.
///
/// Table 254 gives that entry its origin — "[t]he ordinal page number on which this annotation
/// shall appear, where page 0 is the first page" — which is the page *index*, not §7.7.3.3's
/// label and not the object number.
///
/// **The annotations a person added in this session are here too**, and that follows from the
/// save above rather than from a preference: Table 246 has the update performed "just before the
/// submission takes place", so by the time this FDF names the document, the document contains
/// them. Leaving them out would submit a marked-up file beside an FDF that says it is unmarked.
fn annotations(document: &Document, view: &ViewState, owed: &mut Vec<String>) -> Vec<Object> {
    let pages = crate::page::Pages::new(document);
    let mut ordinals: BTreeMap<ObjectId, usize> = BTreeMap::new();
    let mut out = Vec::new();
    let mut dropped = BTreeSet::new();
    let mut truncated = false;
    for ordinal in 0..pages.len() {
        let Some(page) = pages.get(ordinal) else {
            continue;
        };
        if let Some(id) = page.id {
            ordinals.insert(id, ordinal);
        }
        let listed = document.get_key(&page.dict, "Annots");
        let Some(items) = listed.as_array() else {
            continue;
        };
        for item in items {
            let resolved = document.resolve(item);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            if !is_markup(document, dict) {
                continue;
            }
            if out.len() >= MAX_ANNOTATIONS {
                truncated = true;
                break;
            }
            out.push(fdf_annotation(document, dict, ordinal, &mut dropped));
        }
    }
    for added in view.additions() {
        if !is_markup(document, &added.dict) {
            continue;
        }
        let Some(&ordinal) = ordinals.get(&added.page) else {
            owed.push(
                "Table 240 bit 8: an annotation this reader added belongs to a page the page tree \
                 does not reach, so §12.7.8.3.4's required /Page could not be stated for it and \
                 it is not written"
                    .to_owned(),
            );
            continue;
        };
        if out.len() >= MAX_ANNOTATIONS {
            truncated = true;
            break;
        }
        out.push(fdf_annotation(document, &added.dict, ordinal, &mut dropped));
    }
    if truncated {
        owed.push(format!(
            "Table 240 bit 8: this document holds more than {MAX_ANNOTATIONS} markup \
             annotations, which is where the reader of an FDF in this program stops, so the rest \
             are not written"
        ));
    }
    if !dropped.is_empty() {
        let keys: Vec<String> = dropped.into_iter().map(|key| format!("/{key}")).collect();
        owed.push(format!(
            "Table 240 bit 8: {} left out of the annotations written, because each names an \
             object of this document and an FDF has no object space of this document's",
            keys.join(", ")
        ));
    }
    out
}

/// Whether Table 171's `Markup` column says `Yes` of this dictionary's `/Subtype`.
fn is_markup(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| MARKUP.iter().any(|markup| subtype == markup))
}

/// One annotation as §12.7.8.3.4 writes it: its own entries, and Table 254's `/Page`.
///
/// Table 166's `/P` is left out rather than carried, and the two entries are why: `/P` is "[a]n
/// indirect reference to the page object with which this annotation is associated", where
/// `/Page` is an ordinal. An FDF that kept the reference would name an object of a file it is not
/// part of.
fn fdf_annotation(
    document: &Document,
    annotation: &Dictionary,
    ordinal: usize,
    dropped: &mut BTreeSet<String>,
) -> Object {
    let mut out = Dictionary::new();
    for (key, value) in annotation.iter() {
        if key.as_bytes() == b"P" {
            continue;
        }
        match portable(document, value) {
            Some(portable) => {
                out.insert(key.clone(), portable);
            }
            None => {
                dropped.insert(key.escaped());
            }
        }
    }
    out.insert(
        Name::new(&b"Page"[..]),
        Object::Integer(i64::try_from(ordinal).unwrap_or(i64::MAX)),
    );
    Object::Dictionary(out)
}

/// One entry's value, resolved, or `None` where it is or contains something only this document
/// can resolve.
///
/// One hop and no more. Resolving keeps `/Contents 20 0 R` — a string somebody wrote in an
/// indirect object — and refuses `/AP`, whose value resolves to a dictionary of streams that
/// exist nowhere but this file. Following further would mean copying an object graph into a file
/// that has no room for one, and `/Popup` makes that concrete: its `/Parent` points back at the
/// annotation being written.
fn portable(document: &Document, value: &Object) -> Option<Object> {
    let resolved = document.resolve(value);
    (!names_an_object(&resolved, 0)).then_some(resolved)
}

/// Whether this value is, or holds anywhere inside it, something this document alone can resolve.
fn names_an_object(value: &Object, depth: usize) -> bool {
    if depth >= MAX_ENTRY_DEPTH {
        return true;
    }
    match value {
        Object::Reference(_) | Object::Stream(_) => true,
        Object::Array(items) => items
            .iter()
            .any(|item| names_an_object(item, depth.saturating_add(1))),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_, value)| names_an_object(value, depth.saturating_add(1))),
        _ => false,
    }
}

/// One node of Table 249's `/Kids` tree, on the way to being written.
#[derive(Default)]
struct Node {
    /// Table 249's `/V`, where this node is a field with one.
    value: Option<Object>,
    /// §12.7.5.3's selected file, where this node is a file-select control and the document
    /// carries the file it names. Written as `/V`, a §7.11.3 specification around §7.11.4's
    /// embedded file stream, in place of the `value` above.
    file: Option<SelectedFile>,
    /// Whether this node is a field the submission names, as against an ancestor it passes
    /// through on the way to one.
    named: bool,
    /// Children, in first-seen order, by partial name.
    kids: Vec<(String, Node)>,
}

impl Node {
    /// The child with this partial name, made if absent.
    fn kid(&mut self, partial: &str) -> &mut Self {
        let index = if let Some(index) = self.kids.iter().position(|(name, _)| name == partial) {
            index
        } else {
            self.kids.push((partial.to_owned(), Self::default()));
            self.kids.len().saturating_sub(1)
        };
        &mut self.kids[index].1
    }

    /// Table 249's dictionary for this node, allocating an object in `body` for any stream it
    /// needs.
    ///
    /// The body is written to rather than returned from because a file-select control's `/V` is
    /// "a file specification (7.11, "File specifications") identifying the selected file", and a
    /// specification that carries its file carries §7.11.4's *stream* — which §7.3.8 makes an
    /// indirect object, so the tree cannot hold it inline.
    fn dictionary(self, partial: &str, body: &mut Vec<Object>) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"T"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(partial).into()),
        );
        if let Some(file) = self.file {
            let id = ObjectId::new(next_object(body.len()), 0);
            dict.insert(
                Name::new(&b"V"[..]),
                embedded_file_spec(id, file.name.as_deref()),
            );
            body.push(stream(
                embedded_file_dictionary(file.bytes.len(), &file.media_type),
                file.bytes,
            ));
        } else if let Some(value) = self.value {
            dict.insert(Name::new(&b"V"[..]), value);
        }
        if !self.kids.is_empty() {
            dict.insert(
                Name::new(&b"Kids"[..]),
                Object::Array(
                    self.kids
                        .into_iter()
                        .map(|(name, kid)| Object::Dictionary(kid.dictionary(&name, body)))
                        .collect(),
                ),
            );
        }
        dict
    }
}

/// §12.7.8's file, holding these fields and whatever Table 240 asked be carried with them.
///
/// §12.7.8.2.2's header, §12.7.8.2.3's body — Table 245's catalog with its Required `/FDF`,
/// holding Table 246's `/Fields` and, where this document's trailer states one, its `/ID`, plus
/// one indirect object for each stream [`Carried`] holds — §7.5.4's cross-reference table, and
/// §12.7.8.2.4's trailer, "[t]he only required key is Root". The header's version is `1.2`, and
/// that is the only number available rather than a choice between several: the `application/fdf`
/// registration, written by ISO TC 171/SC 2, states that "[o]nly a single version of FDF has ever
/// been defined, which is version 1.2 that was introduced with PDF 1.2".
///
/// **The cross-reference table is written even though §12.7.8.2.1 calls it optional**, and Table
/// 240 bits 7 and 14 are why: both put a stream in the body, bit 14's holds an entire PDF file,
/// and the bytes of that file contain `obj` and `endobj` of its own. A consumer that has to find
/// this file's objects by scanning would find those, so the optional table is what makes the body
/// unambiguous. `startxref` goes with it, since a table nothing points at is a table nobody
/// finds.
fn fdf(
    document: &Document,
    entries: &[Entry],
    carried: &Carried,
    owed: &mut Vec<String>,
) -> Vec<u8> {
    let mut root = Node::default();
    for entry in entries {
        let mut node = &mut root;
        for partial in entry.name.split('.') {
            node = node.kid(partial);
        }
        if node.named {
            owed.push(format!(
                "field {}: named twice by the field table, written once",
                entry.name
            ));
            continue;
        }
        node.named = true;
        node.value.clone_from(&entry.value);
        node.file = entry.file.as_ref().map(|file| SelectedFile {
            name: file.name.clone(),
            media_type: file.media_type.clone(),
            bytes: file.bytes.clone(),
        });
    }
    // The body's indirect objects after the catalog, which is object 1. Written to first by the
    // field tree, whose file-select controls each need §7.11.4's stream.
    let mut body: Vec<Object> = Vec::new();
    let mut fdf = Dictionary::new();
    fdf.insert(
        Name::new(&b"Fields"[..]),
        Object::Array(
            root.kids
                .into_iter()
                .map(|(name, kid)| Object::Dictionary(kid.dictionary(&name, &mut body)))
                .collect(),
        ),
    );
    // Table 246's `/ID`: "taken from the ID entry in the file's trailer dictionary", where that
    // trailer states §14.4's pair of strings.
    if let Some(Object::Array(identifier)) = document.trailer().get("ID")
        && let [first, second] = identifier.as_slice()
        && first.as_string().is_some()
        && second.as_string().is_some()
    {
        fdf.insert(
            Name::new(&b"ID"[..]),
            Object::Array(vec![first.clone(), second.clone()]),
        );
    }
    if !carried.annotations.is_empty() {
        fdf.insert(
            Name::new(&b"Annots"[..]),
            Object::Array(carried.annotations.clone()),
        );
    }

    if let Some(differences) = &carried.differences {
        let id = ObjectId::new(next_object(body.len()), 0);
        fdf.insert(Name::new(&b"Differences"[..]), Object::Reference(id));
        body.push(stream(Dictionary::new(), differences.clone()));
    }
    if let Some(file) = &carried.embedded {
        let id = ObjectId::new(next_object(body.len()), 0);
        fdf.insert(Name::new(&b"F"[..]), embedded_file_spec(id, None));
        body.push(stream(
            embedded_file_dictionary(file.len(), "application/pdf"),
            file.clone(),
        ));
    }

    let mut out = b"%FDF-1.2\n".to_vec();
    let mut offsets = Vec::with_capacity(body.len().saturating_add(1));
    for (index, object) in std::iter::once(&Object::Dictionary({
        let mut catalog = Dictionary::new();
        catalog.insert(Name::new(&b"FDF"[..]), Object::Dictionary(fdf));
        catalog
    }))
    .chain(body.iter())
    .enumerate()
    {
        offsets.push(out.len());
        let mut header = String::new();
        let _ = writeln!(header, "{} 0 obj", index.saturating_add(1));
        out.extend_from_slice(header.as_bytes());
        pdf_syntax::write::object(object, &mut out);
        out.extend_from_slice(b"\nendobj\n");
    }
    let table_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut tail = String::new();
    // §7.5.4: the first entry of the first subsection "shall be the one with object number 0",
    // "shall always have a generation number of 65,535" and "shall be the head of the linked list
    // of free objects".
    let _ = write!(tail, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Root 1 0 R /Size {size} >>\nstartxref\n{table_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

/// The object number of the next thing written after the catalog, which is object 1.
fn next_object(written: usize) -> u32 {
    u32::try_from(written).unwrap_or(u32::MAX).saturating_add(2)
}

/// One indirect stream object, with the `/Length` §7.3.8.2 makes required of one.
///
/// > The number of bytes from the beginning of the line following the keyword stream to the
/// > last byte just before the keyword endstream .
///
/// No `/Filter`: what these two streams carry is a PDF file and part of one, and compressing a
/// body that a server is about to read is a trade nobody here has measured.
fn stream(mut dict: Dictionary, data: Vec<u8>) -> Object {
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(Arc::new(Stream {
        dict,
        data: Arc::from(data),
        decryption_failed: false,
    }))
}

/// A §7.11.3 file specification around §7.11.4's embedded file stream, for the two entries that
/// ask for one.
///
/// Table 240 bit 14's `/F` is "a file specification containing an embedded file stream
/// representing the PDF file from which the FDF is being submitted", and §12.7.5.3's file-select
/// control's `/V` is "a file specification (7.11, "File specifications") identifying the selected
/// file". One shape answers both.
///
/// §7.11.3's dictionary form, with Table 43's `/Type` — "[r]equired if an EF, EP or RF entry is
/// present" — and the `/EF` dictionary whose value "shall be an embedded file stream (see 7.11.4,
/// "Embedded file streams") containing the corresponding file".
///
/// **Table 43's own `/F` is written only where the caller knows a name**, and the two callers
/// differ for a reason rather than by omission. The entry is "[r]equired if the DOS, Mac, and Unix
/// entries are all absent", and it is "[a] file specification string of the form described in
/// 7.11.2" — a *path*. For bit 14 that path is the machine this program is running on and this
/// crate has none: `pdf_syntax::FileBytes` keeps the bytes and not the name it read them under,
/// deliberately, because `viewer_core`'s rule 2 gives the layers above no filesystem either, and
/// [`Carried::save`] owes the sentence. For a file-select control the name is the *document's* —
/// what its own specification said the selected file is called — so it is carried on rather than
/// invented. `/UF` goes with it, which Table 43 requires a reader prefer.
fn embedded_file_spec(stream: ObjectId, name: Option<&str>) -> Object {
    let mut embedded = Dictionary::new();
    embedded.insert(Name::new(&b"F"[..]), Object::Reference(stream));
    let mut spec = Dictionary::new();
    spec.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Filespec"[..])),
    );
    if let Some(name) = name {
        spec.insert(
            Name::new(&b"F"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(name).into()),
        );
        spec.insert(
            Name::new(&b"UF"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(name).into()),
        );
    }
    spec.insert(Name::new(&b"EF"[..]), Object::Dictionary(embedded));
    Object::Dictionary(spec)
}

/// Table 44's entries for the stream that `/EF` names.
///
/// `/Subtype` is the media type, which the table requires be one: "[t]he value of this entry
/// shall conform to the MIME media type names defined in Internet RFC 2046, with the provision
/// that characters not permitted in names shall use the 2-character hexadecimal code format
/// described in 7.3.5". Table 240 bit 9 states PDF's own — `application/pdf` — and a file-select
/// control's file states whatever its own `/Subtype` said; §7.3.5 is what turns a SOLIDUS into
/// `#2F`, inside `Name::escaped` rather than here.
///
/// Table 45's `/Size` is "[t]he size of the uncompressed embedded file, in bytes", which is the
/// data's own length because nothing here writes a `/Filter`.
fn embedded_file_dictionary(size: usize, media_type: &str) -> Dictionary {
    let mut params = Dictionary::new();
    params.insert(
        Name::new(&b"Size"[..]),
        Object::Integer(i64::try_from(size).unwrap_or(i64::MAX)),
    );
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"EmbeddedFile"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(media_type.as_bytes())),
    );
    dict.insert(Name::new(&b"Params"[..]), Object::Dictionary(params));
    dict
}

/// HTML 4.01 section 17.13.4's `application/x-www-form-urlencoded`, over these fields.
///
/// That definition, which §12.7.6.2 points at by naming the specification. It writes each
/// literal between a GRAVE ACCENT and an APOSTROPHE, and they are rustdoc code spans here — the
/// same substitution of markup this project's own quotation checker makes on both sides:
///
/// 1. "Control names and values are escaped. Space characters are replaced by `+`, and then
///    reserved characters are escaped as described in [RFC1738], section 2.2: Non-alphanumeric
///    characters are replaced by `%HH`, a percent sign and two hexadecimal digits representing
///    the ASCII code of the character. Line breaks are represented as "CR LF" pairs (i.e.,
///    `%0D%0A`)."
/// 2. "The control names/values are listed in the order they appear in the document. The name is
///    separated from the value by `=` and name/value pairs are separated from each other by `&`."
///
/// The bytes escaped are UTF-8, which is Table 239's `/CharSet` default and the one value of it
/// this writes. The order is the field table's — §12.7.4.2's names, sorted — rather than the
/// document's, which is a choice recorded as one: the clause states no order and HTML's is about
/// a page's controls, which a field tree does not have.
///
/// A field with several values — §12.7.5.4's multiple selection, an array — is written as one
/// pair per value under the same name. **This is a choice and nothing more.** §12.7.6.2 states
/// nothing about it, and the specification it names states the encoding of a *control* without
/// saying what a control with several chosen values contributes; repeating the name is the only
/// shape the encoding above leaves available, since it has no syntax for a list.
fn urlencoded(document: &Document, entries: &[Entry], owed: &mut Vec<String>) -> String {
    let mut out = String::new();
    for entry in entries {
        let values: Vec<Vec<u8>> = match &entry.value {
            None => vec![Vec::new()],
            Some(Object::Array(items)) => items
                .iter()
                .filter_map(|item| text_bytes(document, item))
                .collect(),
            Some(value) => {
                if let Some(bytes) = text_bytes(document, value) {
                    vec![bytes]
                } else {
                    owed.push(format!(
                        "field {}: its value is {}, which no field type gives a text value, so it \
                         is not written",
                        entry.name,
                        value.type_name()
                    ));
                    continue;
                }
            }
        };
        for value in values {
            if !out.is_empty() {
                out.push('&');
            }
            escape(entry.name.as_bytes(), &mut out);
            out.push('=');
            escape(&value, &mut out);
        }
    }
    out
}

/// HTML 4.01 section 17.13.4's escaping of one name or value, appended.
fn escape(bytes: &[u8], out: &mut String) {
    let mut previous = 0u8;
    for &byte in bytes {
        match byte {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => out.push(char::from(byte)),
            b' ' => out.push('+'),
            // "Line breaks are represented as "CR LF" pairs": a lone LF becomes the pair, and an
            // LF already after a CR is the pair's own second half.
            b'\n' if previous != b'\r' => out.push_str("%0D%0A"),
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
        previous = byte;
    }
}

/// How many delimiters this composition tries before it refuses to write a multipart body.
///
/// HTML 4.01 section 17.13.4.2 leaves the choice open — "[p]art boundaries should not occur in
/// any of the data; how this is done lies outside the scope of this specification" — so the bound
/// is this program's and is written down as one. Each candidate is a distinct string, so a body
/// that defeats all of them is a body constructed to; the alternative to a bound is scanning the
/// whole body once per candidate, which a document could make quadratic by embedding a file.
const MAX_BOUNDARY_TRIES: u32 = 1024;

/// §12.7.5.3's body for a form holding a file-select control, with the delimiter it was written
/// with.
///
/// > For fields submitted in HTML Form format, the submission shall use the MIME content type
/// > multipart / form-data, as described in Internet RFC 2045.
///
/// ISO 32000-2 names RFC 2045 and stops there; what says how a *form* uses that media type is the
/// specification §12.7.6.2 names for the format itself, "HTML Form format (described in the HTML
/// 4.01 Specification)". Its section 17.13.4.2 defines the body as a series of parts, "each
/// representing a successful control", sent "in the same order the corresponding controls appear
/// in the document stream" — which here is the order [`chosen`] read the fields in.
///
/// Each part carries "a "Content-Disposition" header whose value is "form-data"" and "a name
/// attribute specifying the control name of the corresponding control"; a part carrying a file
/// adds the section's `filename` parameter — "[t]he user agent should attempt to supply a file
/// name for each submitted file" — and its content type, since "the file input should be
/// identified by the appropriate content type". RFC 2046 section 5.1.1 supplies the delimiter
/// line: "two hyphen characters … followed by the boundary parameter value … and a terminating
/// CRLF", with the last one followed by two more hyphens. Lines are CRLF throughout, which
/// section 17.13.4.2 states outright: "As with all MIME transmissions, "CR LF" … is used to
/// separate lines of data."
///
/// **No `Content-Transfer-Encoding` and no `charset`.** The section makes both optional — a part
/// "may be encoded and the "Content-Transfer-Encoding" header supplied if the value of that part
/// does not conform to the default (7BIT) encoding" — and this body is handed to a host as bytes
/// over a transport that carries bytes, so encoding them into a subset and back would lose the
/// exactness the file's own `/EF` stream has. A text part's bytes are the UTF-8 the rest of this
/// module writes, which Table 239's `/CharSet` sentence on [`compose`] already reports.
///
/// # Errors
///
/// [`Refusal::Undelimitable`], for a body that contains every delimiter [`MAX_BOUNDARY_TRIES`]
/// would try.
fn multipart(
    document: &Document,
    entries: &[Entry],
    placed: Option<&Placed>,
    owed: &mut Vec<String>,
) -> Result<(String, Vec<u8>), Refusal> {
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    for entry in entries {
        let Some(name) = header_quoted(&entry.name) else {
            owed.push(format!(
                "field {}: its name holds a line break, which no part header can carry, so the \
                 field is left out of the multipart body rather than renamed",
                entry.name
            ));
            continue;
        };
        if let Some(file) = &entry.file {
            let filename = match file.name.as_deref().map(header_quoted) {
                Some(Some(filename)) => format!("; filename=\"{filename}\""),
                // §7.11.3's specification need not name the file it carries, and HTML 4.01
                // section 17.13.4.2's file name is a `should` rather than a requirement.
                Some(None) | None => String::new(),
            };
            parts.push((
                format!(
                    "Content-Disposition: form-data; name=\"{name}\"{filename}\r\nContent-Type: \
                     {}\r\n",
                    file.media_type
                ),
                file.bytes.clone(),
            ));
            continue;
        }
        let values: Vec<Vec<u8>> = match &entry.value {
            None => vec![Vec::new()],
            Some(Object::Array(items)) => items
                .iter()
                .filter_map(|item| text_bytes(document, item))
                .collect(),
            Some(value) => {
                let Some(bytes) = text_bytes(document, value) else {
                    owed.push(format!(
                        "field {}: its value is {}, which no field type gives a text value, so it \
                         is not written",
                        entry.name,
                        value.type_name()
                    ));
                    continue;
                };
                vec![bytes]
            }
        };
        for value in values {
            parts.push((
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n"),
                value,
            ));
        }
    }
    if let Some(placed) = placed {
        for (suffix, coordinate) in [("x", placed.across), ("y", placed.down)] {
            let name = match &placed.named {
                Some(named) => format!("{named}.{suffix}"),
                None => suffix.to_owned(),
            };
            match header_quoted(&name) {
                Some(name) => parts.push((
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n"),
                    coordinate.to_string().into_bytes(),
                )),
                None => owed.push(
                    "Table 240 bit 5's SubmitCoordinates: the name it writes the coordinates \
                     under holds a line break, which no part header can carry"
                        .to_owned(),
                ),
            }
        }
    }

    let boundary = boundary(&parts).ok_or(Refusal::Undelimitable)?;
    let mut body = Vec::new();
    for (headers, data) in &parts {
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(headers.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");
    Ok((boundary, body))
}

/// A delimiter none of the parts contains, or `None` where every candidate occurs in one.
///
/// Deterministic, because a body that differs run to run is a body no test can hold to anything.
/// The characters are RFC 2046 section 5.1.1's `bcharsnospace`, and the longest candidate is far
/// inside that section's `0*69<bchars> bcharsnospace`.
fn boundary(parts: &[(String, Vec<u8>)]) -> Option<String> {
    (0..MAX_BOUNDARY_TRIES).find_map(|attempt| {
        let candidate = format!("quorra-form-data-{attempt}");
        let delimiter = format!("--{candidate}");
        let occurs = parts.iter().any(|(headers, data)| {
            headers.contains(&delimiter)
                || data
                    .windows(delimiter.len())
                    .any(|window| window == delimiter.as_bytes())
        });
        (!occurs).then_some(candidate)
    })
}

/// One name as a quoted string, or `None` for text no header field can hold.
///
/// RFC 2045 section 5.1 writes a parameter value as `value := token / quoted-string` and lists
/// the REVERSE SOLIDUS and the QUOTATION MARK among the `tspecials` that "[m]ust be in
/// quoted-string, to use within parameter values"; the quoted string itself is RFC 822's, where a
/// `quoted-pair` is a REVERSE SOLIDUS before the character it protects. So both are escaped that
/// way. A carriage return, a line feed or a NUL is not escapable at all — a header field ends at
/// the line break — so a name holding one is reported by its caller rather than silently
/// shortened into a different name.
fn header_quoted(text: &str) -> Option<String> {
    if text.contains(['\r', '\n', '\0']) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '\\' || character == '"' {
            out.push('\\');
        }
        out.push(character);
    }
    Some(out)
}

/// Table 240 bit 5's coordinates, appended to the query. §12.7.6.2 states the whole rule:
///
/// > If set, the coordinates of the mouse click that caused the submit-form action shall be
/// > transmitted as part of the form data. The coordinate values are relative to the upper-left
/// > corner of the field's widget annotation rectangle. They shall be represented in the data in
/// > the format name . x = xval & name . y = yval where name is the field's mapping name ( TM in
/// > the field dictionary) if present; otherwise, name is the field name. If the value of the TM
/// > entry is a single ASCII SPACE (20h) character, both the name and the ASCII PERIOD (2Eh)
/// > following it shall be suppressed, resulting in the format x = xval & y = yval
///
/// Rounded to integers the way §12.6.4.8's `/IsMap` coordinates are, and for the same reason: the
/// table states no precision and a pixel is the unit a click has.
///
/// The two pairs come back rather than a piece of query string, because the format the table
/// states — `name.x=xval&name.y=yval` — is *two name/value pairs* and the `&` and `=` are how
/// HTML 4.01 section 17.13.4.1's `application/x-www-form-urlencoded` writes a pair down. The same
/// two pairs are two parts in a multipart body, and neither caller should have to unpick the
/// other's punctuation to get at them.
fn coordinates(
    document: &Document,
    table: &BTreeMap<String, Vec<ObjectId>>,
    click: Option<&Click>,
    owed: &mut Vec<String>,
) -> Option<Placed> {
    let Some(click) = click else {
        owed.push(
            "Table 240 bit 5's SubmitCoordinates: the action was not invoked by a click, so there \
             are no coordinates to transmit"
                .to_owned(),
        );
        return None;
    };
    let (x, y) = click.point;
    // "the field's widget annotation rectangle", read from the annotation the click was over
    // rather than taken from the caller: §12.6.4.8's `/IsMap` measures from a different one.
    let rect = click
        .widget
        .and_then(|widget| document.get(widget).as_dict().cloned())
        .and_then(|dict| crate::annotation::rectangle(document, &dict, "Rect"));
    let Some([llx, _, _, ury]) = rect else {
        owed.push(
            "Table 240 bit 5's SubmitCoordinates: the click was over no widget annotation with a \
             /Rect, so there is no rectangle to measure the coordinates from"
                .to_owned(),
        );
        return None;
    };
    let across = (x - llx).round();
    let down = (ury - y).round();
    if !across.is_finite() || !down.is_finite() {
        owed.push(
            "Table 240 bit 5's SubmitCoordinates: the click names no finite position".to_owned(),
        );
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "rounded and finite, and a page is bounded by §14.11.2's 14 400 units"
    )]
    let (across, down) = (across as i64, down as i64);

    // The field's mapping name, up §12.7.4.1's inheritance: Table 226 makes `/TM` inheritable.
    let widget = click.widget;
    let field_name = widget.and_then(|widget| {
        table
            .iter()
            .find(|(_, widgets)| widgets.contains(&widget))
            .map(|(name, _)| name.clone())
    });
    let mapping = widget
        .and_then(|widget| document.get(widget).as_dict().cloned())
        .and_then(|dict| mapping_name(document, &dict));
    let named = match (mapping, field_name) {
        // "If the value of the TM entry is a single ASCII SPACE (20h) character, both the name
        // and the ASCII PERIOD (2Eh) following it shall be suppressed".
        (Some(mapping), _) if mapping == " " => None,
        (Some(mapping), _) => Some(mapping),
        (None, Some(name)) => Some(name),
        (None, None) => {
            owed.push(
                "Table 240 bit 5's SubmitCoordinates: the widget pressed belongs to no named \
                 field, so the coordinates are written without a name"
                    .to_owned(),
            );
            None
        }
    };
    Some(Placed {
        named,
        across,
        down,
    })
}

/// Table 240 bit 5's click, as the two name/value pairs the table's format states.
struct Placed {
    /// What goes before the full stop, `x` and `y`: the mapping name or the field name, or
    /// `None` where the table's single-space `/TM` suppresses "both the name and the ASCII
    /// PERIOD (2Eh) following it". Unescaped, because each body writes a name its own way.
    named: Option<String>,
    /// `xval`, across from the widget rectangle's left edge.
    across: i64,
    /// `yval`, down from its upper edge.
    down: i64,
}

/// Table 226's `/TM`, "[t]he mapping name that shall be used when exporting interactive form
/// field data from the document", from the nearest dictionary up the `/Parent` chain that states
/// it.
fn mapping_name(document: &Document, widget: &Dictionary) -> Option<String> {
    let mut current = widget.clone();
    for _ in 0..crate::appearance::MAX_FIELD_ANCESTRY {
        if let Object::String(bytes) = document.get_key(&current, "TM") {
            return Some(pdf_syntax::text_string(&bytes));
        }
        current = document.get_key(&current, "Parent").as_dict()?.clone();
    }
    None
}

/// The URL with the query appended, for an HTTP GET.
///
/// **A deliberate departure from the specification §12.7.6.2 names, in one case only.** HTML 4.01
/// section 17.13.3 says the user agent "takes the value of action, appends a `?` to it, then
/// appends the form data set" — unconditionally, so an action that already carries a query would
/// get a second `?`. RFC 3986 section 3.4 says a URI has one query and that it is "indicated by
/// the first question mark ("?") character", so that second `?` is data inside the first query
/// rather than a separator, and the server reads the field names of a submission as part of a
/// value. The form data is joined with `&` instead, which is the separator the same encoding uses
/// between its own pairs and which leaves every name a name. Table 239 types `/F` as a URL and
/// says nothing about whether it may carry a query, so this is a choice about malformed-adjacent
/// input rather than a rule either document states.
fn with_query(url: &str, query: &str) -> String {
    if query.is_empty() {
        return url.to_owned();
    }
    let joiner = if url.contains('?') { '&' } else { '?' };
    format!("{url}{joiner}{query}")
}

#[cfg(test)]
mod tests {
    use super::{escape, with_query};

    /// HTML 4.01 section 17.13.4, character class by character class.
    #[test]
    fn html_forms_escaping_keeps_alphanumerics_and_replaces_the_rest() {
        let mut out = String::new();
        escape("a b&c=d\né\r\n".as_bytes(), &mut out);
        assert_eq!(out, "a+b%26c%3Dd%0D%0A%C3%A9%0D%0A");
    }

    #[test]
    fn a_query_joins_an_existing_one_rather_than_starting_a_second() {
        assert_eq!(with_query("https://h/p", "a=1"), "https://h/p?a=1");
        assert_eq!(with_query("https://h/p?k=v", "a=1"), "https://h/p?k=v&a=1");
        assert_eq!(with_query("https://h/p", ""), "https://h/p");
    }
}
