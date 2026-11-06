//! An `event` that submits the form or calls a web service instead of running a script: two of
//! XFA 3.3 page 703's one-of properties, `submit` and `execute`, read from the template and carried
//! out as far as this reader's own structures reach (ADR 2738).
//!
//! **A `submit` is §12.7.6.2's submission stated in the template.** Page 858 describes the element
//! as the instructions for submitting the form's data to a host by an HTTP POST, and three of the
//! six packagings page 859 lists for its `format` are bodies [`crate::submission::compose`] already
//! writes for a submit-form action: `pdf`, the data packaged as PDF, is Table 240 bit 9's whole
//! document; `xfd`, the data packaged as XFDF, is bit 6's; and `urlencoded`, the data URL-encoded
//! as RFC 1738 has it, is bit 3's HTML Form format. So [`Submit::action`] states the `submit` as
//! the submit-form action whose flags ask for the same body, and the composition, the reader's
//! submit level (`doc/questions/A98`) and page 446's pre- and post-submit steps are the ones a
//! submit-form action already has. In a conforming hybrid file the field's widget carries no `A`
//! of its own where the XFA resource gives the field actions (Annex K), so the template's
//! `submit` is the only statement of the submit the file makes.
//!
//! What is not composed is named where the event fires, and nothing is sent:
//!
//! - `xdp`, which page 597's convention makes the default, and `xml`: chapter 23's packaging and
//!   chapter 15's data unloading, which this reader does not write.
//! - `formdata`: page 859 says only that it emulates the URL-encoding of certain legacy software
//!   and names neither the software nor the encoding, so no body can be derived from it.
//! - A `submit` holding `signData`, `encrypt` or `encryptData`: page 453's client steps 3 and 4
//!   sign and encrypt the content before it is posted, and this reader makes no XML digital
//!   signature and performs no XML encryption, so sending the content would send what the form
//!   did not ask to have sent.
//! - A `submit` with no `target`: page 860 leaves the URI to a technique of the product's own, and
//!   this reader has none.
//!
//! **An `execute` calls a WSDL web service** (pages 722–723) through a connection the
//! `connectionSet` packet defines, and this reader calls none: one whose `runAt` is `server` is
//! one page 722 tells a client to ignore, and runs nothing, which is the whole of what the client
//! owes it; any other is named.

use super::xml::{Element, Tree};
use crate::action::{SubmitFlags, SubmitForm};

/// Table 240 bit 2, `IncludeNoValueFields`.
const INCLUDE_NO_VALUE_FIELDS: u32 = 1 << 1;
/// Table 240 bit 3, `ExportFormat`: HTML Form format.
const EXPORT_FORMAT: u32 = 1 << 2;
/// Table 240 bit 6, `XFDF`.
const XFDF: u32 = 1 << 5;
/// Table 240 bit 9, `SubmitPDF`.
const SUBMIT_PDF: u32 = 1 << 8;

/// An `event`'s `submit` property (XFA 3.3 pages 858–861), as the template states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Submit {
    /// Its `format`, trimmed; `xdp` where it states none, the value page 858 underlines, which
    /// page 597 says is the one an omitted attribute takes.
    format: String,
    /// Its `target`, trimmed, where it states a non-blank one.
    target: Option<String>,
    /// Its `textEncoding`, trimmed, where it states a non-blank one.
    text_encoding: Option<String>,
    /// Whether it holds a `signData` child (page 858), which page 453's step 3 performs.
    signs: bool,
    /// Whether it holds an `encrypt` or an `encryptData` child (page 858), which page 453's
    /// step 4 performs.
    encrypts: bool,
}

/// What a [`Submit`] comes to: the submit-form action that composes the body it asks for, and
/// what of it that composition does not do, or the sentence saying why nothing is sent.
pub(crate) type Composed = Result<(SubmitForm, Vec<String>), String>;

impl Submit {
    /// Reads the `submit` element at `at`.
    pub(crate) fn read(tree: &Tree, at: usize, submit: &Element) -> Self {
        let stated = |name: &str| {
            submit
                .attribute(name)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        };
        Self {
            format: stated("format").unwrap_or_else(|| "xdp".to_owned()),
            target: stated("target"),
            text_encoding: stated("textEncoding"),
            signs: tree.child(at, "signData").is_some(),
            encrypts: tree.child(at, "encrypt").is_some()
                || tree.child(at, "encryptData").is_some(),
        }
    }

    /// The submit-form action that composes the body this `submit` asks for, with what that
    /// composition leaves undone, or why nothing is sent (the module's list).
    ///
    /// The flags: bit 9, 6 or 3 for the format, as the module says; bit 4 clear, since page 858
    /// submits by an HTTP POST; and bit 2, `IncludeNoValueFields`, because page 445 makes a
    /// submission carry the form's variable data, of which a field holding nothing is a part, where
    /// a submit-form action without the bit leaves such a field out. `/Fields` is empty, so every
    /// field the form holds goes, but those Table 227's `NoExport` keeps back.
    pub(crate) fn action(&self) -> Composed {
        if self.signs {
            return Err(
                "holds a signData, which page 453 performs before the content is sent, and this \
                 reader makes no XML digital signature, so nothing is sent"
                    .to_owned(),
            );
        }
        if self.encrypts {
            return Err(
                "holds an encrypt or an encryptData, which page 453 performs before the content \
                 is sent, and this reader performs no XML encryption, so nothing is sent"
                    .to_owned(),
            );
        }
        let flags = match self.format.as_str() {
            "pdf" => SUBMIT_PDF,
            "xfd" => XFDF,
            "urlencoded" => EXPORT_FORMAT,
            "xdp" | "xml" => {
                return Err(format!(
                    "asks for the {} packaging (page 859), which this reader does not write, so \
                     nothing is sent",
                    self.format
                ));
            }
            "formdata" => {
                return Err(
                    "asks for the formdata packaging, which page 859 describes only as emulating \
                     legacy software it does not name, so nothing is sent"
                        .to_owned(),
                );
            }
            other => {
                return Err(format!(
                    "asks for a format {other} that page 858 does not list, so nothing is sent"
                ));
            }
        };
        let Some(url) = self.target.clone() else {
            return Err(
                "states no target, which page 860 leaves to a technique of the product's own, and \
                 this reader has none, so nothing is sent"
                    .to_owned(),
            );
        };
        let mut owed = Vec::new();
        // Page 860: the encoding a body's text is in. A PDF states its own; the other two are
        // written in UTF-8, so another encoding the template asks for is said rather than applied.
        if flags != SUBMIT_PDF
            && let Some(encoding) = &self.text_encoding
            && !encoding.eq_ignore_ascii_case("UTF-8")
        {
            owed.push(format!(
                "the submit's textEncoding {encoding} (page 860) is not applied; its text is \
                 written in UTF-8"
            ));
        }
        Ok((
            SubmitForm {
                url,
                fields: Vec::new(),
                flags: SubmitFlags(flags | INCLUDE_NO_VALUE_FIELDS),
                charset: None,
            },
            owed,
        ))
    }
}

/// An `event`'s `execute` property (XFA 3.3 pages 722–723), as far as a client reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Execute {
    /// Its `connection`, trimmed: the name of the connection in the `connectionSet` packet.
    connection: String,
    /// Whether its `runAt` is `server`, which page 722 tells a client to ignore. Page 722
    /// underlines `client` as the default.
    server_only: bool,
}

impl Execute {
    /// Reads the `execute` element `execute`.
    pub(crate) fn read(execute: &Element) -> Self {
        Self {
            connection: execute
                .attribute("connection")
                .map(str::trim)
                .unwrap_or_default()
                .to_owned(),
            server_only: execute
                .attribute("runAt")
                .is_some_and(|run_at| run_at.trim() == "server"),
        }
    }

    /// The sentence saying why the client does not invoke the service, or `None` where page 722
    /// tells it to ignore the `execute`, which does nothing and is not reported.
    pub(crate) fn unperformed(&self) -> Option<String> {
        if self.server_only {
            return None;
        }
        Some(if self.connection.is_empty() {
            "names no connection, which page 722 makes an error".to_owned()
        } else {
            format!(
                "calls the web service of the connection {} (page 722), which this reader does \
                 not call",
                self.connection
            )
        })
    }
}
