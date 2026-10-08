//! The host object model's refused members, by name: RFC 0008 section 4.3's Tier 2 with each row's
//! reason, what section 4.2 admits to Tier 1 that this program refuses with a reason of its own, and
//! what section 4.2 admits that this bridge does not yet carry.
//!
//! Data, so that a test can walk it and a later round can move a member from one list to the
//! bridge by deleting one line here. Each member becomes a property that throws a
//! `NotAllowedError` naming it (ADR 1591); a member in neither list and not carried is simply not
//! there, which is how the reference's surface ends.

/// The object a member is a property of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    /// The document, `this` in a field script — the global object here, so `getField` and
    /// `this.getField` are one property.
    Doc,
    /// `app`.
    App,
    /// `event`.
    Event,
    /// A `Field`: `event.target`, and what `this.getField` answers.
    Field,
    /// `util`.
    Util,
    /// `console`.
    Console,
    /// A name of the global scope that is an object of its own: `Net`, `color`.
    Global,
    /// `global`, the document's own store of values (ADR 1626).
    Store,
    /// An `OCG`, what `this.getOCGs` answers.
    Layer,
}

impl Holder {
    /// How a script spells the holder in front of a member, `app.` or `this.`; empty for
    /// [`Holder::Global`], whose members are names on their own.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Doc => "this.",
            Self::App => "app.",
            Self::Event => "event.",
            Self::Field => "Field.",
            Self::Util => "util.",
            Self::Console => "console.",
            Self::Global => "",
            Self::Store => "global.",
            Self::Layer => "OCG.",
        }
    }
}

/// One row of RFC 0008 section 4.3's table: members excluded for one reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Excluded {
    /// The object the members are properties of.
    pub holder: Holder,
    /// The members.
    pub members: &'static [&'static str],
    /// The table's reason, as the clause after "because it".
    pub reason: &'static str,
}

/// RFC 0008 section 4.3's Tier 2, row by row.
pub const EXCLUDED: &[Excluded] = &[
    Excluded {
        holder: Holder::Global,
        members: &["Net", "SOAP"],
        reason: "leaves the process: the confined worker has no network, and the host makes no \
                 transmission a person did not see",
    },
    Excluded {
        holder: Holder::App,
        members: &["launchURL"],
        reason: "leaves the machine: it is a link's question, under the host's level for links, \
                 and a script may raise it only once the host carries a script's request",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["submitForm"],
        reason: "is a submission, under the host's level for submissions, and a script may ask \
                 for one only once the host carries a script's request",
    },
    Excluded {
        holder: Holder::Field,
        members: &["browseForFileToSubmit", "buttonImportIcon"],
        reason: "opens a file chooser, which is a person's act",
    },
    Excluded {
        holder: Holder::Doc,
        members: &[
            "saveAs",
            "exportDataObject",
            "importDataObject",
            "createDataObject",
            "getDataObjectContents",
            "exportAsFDF",
            "exportAsXFDF",
            "exportAsText",
            "importAnFDF",
            "importAnXFDF",
            "importTextData",
            "importXFAData",
            "exportXFAData",
        ],
        reason: "reaches the filesystem, which the confined worker does not have",
    },
    Excluded {
        holder: Holder::App,
        members: &["browseForDoc", "getPath"],
        reason: "reaches the filesystem, which the confined worker does not have",
    },
    Excluded {
        holder: Holder::Util,
        members: &["readFileIntoStream"],
        reason: "reaches the filesystem, which the confined worker does not have",
    },
    Excluded {
        holder: Holder::App,
        members: &["openDoc", "newDoc", "newFDF", "openFDF"],
        reason: "reaches other documents in the window, which one document's script may not",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["closeDoc"],
        reason: "reaches other documents in the window, which one document's script may not",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["print", "getPrintParams"],
        reason: "prints, which RFC 0004 builds under its own policy",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["mailDoc", "mailForm"],
        reason: "starts a mail program, which nothing here starts",
    },
    Excluded {
        holder: Holder::App,
        members: &["mailMsg", "mailGetAddrs"],
        reason: "starts a mail program, which nothing here starts",
    },
    Excluded {
        holder: Holder::App,
        members: &[
            "execMenuItem",
            "addMenuItem",
            "addSubMenu",
            "hideMenuItem",
            "addToolButton",
            "removeToolButton",
            "hideToolbarButton",
            "popUpMenu",
            "popUpMenuEx",
            "execDialog",
        ],
        reason: "draws or drives the application's own chrome, which a document does not",
    },
    // Adobe's "FullScreen" page makes `app.fs` the application's full-screen preferences, and RFC
    // 0008 section 4.2 admits no member of it to `app`; the document's own request for full screen
    // is the catalog's `/PageMode`, which the host reads (ADR 1665).
    Excluded {
        holder: Holder::App,
        members: &["fs"],
        reason: "sets the application's full-screen preferences, which a document does not; a \
                 document asks for full-screen mode through its catalog's /PageMode",
    },
    Excluded {
        holder: Holder::App,
        members: &[
            "trustedFunction",
            "trustPropagatorFunction",
            "beginPriv",
            "endPriv",
        ],
        reason: "enters a privileged context, which a document's events do not have",
    },
    Excluded {
        holder: Holder::Global,
        members: &["Collab", "security", "SecurityHandler", "identity"],
        reason: "reaches collaboration servers, credentials, the reader's identity or \
                 cryptography the document steers",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["encryptUsingPolicy"],
        reason: "reaches collaboration servers, credentials, the reader's identity or \
                 cryptography the document steers",
    },
    Excluded {
        holder: Holder::Field,
        members: &[
            "signatureSign",
            "signatureValidate",
            "signatureInfo",
            "signatureGetSeedValue",
            "signatureSetSeedValue",
            "signatureGetModifications",
            "setLock",
            "getLock",
        ],
        reason: "reaches collaboration servers, credentials, the reader's identity or \
                 cryptography the document steers",
    },
    Excluded {
        holder: Holder::Doc,
        members: &[
            "addScript",
            "removeScript",
            "addField",
            "removeField",
            "addAnnot",
            "addLink",
            "insertPages",
            "deletePages",
            "replacePages",
            "extractPages",
            "flattenPages",
            "spawnPageFromTemplate",
            "addWatermarkFromFile",
            "addWatermarkFromText",
            "setPageAction",
            "setAction",
        ],
        reason: "changes the document's structure or invents marks, and the document is \
                 immutable",
    },
    Excluded {
        holder: Holder::Field,
        members: &["setAction"],
        reason: "changes the document's structure or invents marks, and the document is \
                 immutable",
    },
    Excluded {
        holder: Holder::Store,
        members: &["setPersistent", "subscribe"],
        reason: "carries a value across documents and sessions, which is a channel between files",
    },
    Excluded {
        holder: Holder::Layer,
        members: &["setAction", "setIntent"],
        reason: "changes the document's structure or invents marks, and the document is \
                 immutable",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["media"],
        reason: "is multimedia, which clause 13's exclusion keeps out",
    },
    Excluded {
        holder: Holder::App,
        members: &["media"],
        reason: "is multimedia, which clause 13's exclusion keeps out",
    },
    Excluded {
        holder: Holder::Global,
        members: &["XFA", "xfa"],
        reason: "is XFA, which Annex K permits a processor not to implement",
    },
    Excluded {
        holder: Holder::Doc,
        members: &["dynamicXFAForm"],
        reason: "is XFA, which Annex K permits a processor not to implement",
    },
];

/// What RFC 0008 section 4.2 admits to Tier 1 and this program refuses by name, each row with the
/// reason, as the clause after "because it" (ADRs 1724, 1725).
///
/// Not Tier 2 — the RFC admits each — and not a member waiting to be bridged: each reaches
/// something this program keeps out of a document's realm, or that no reader's edit reaches, and
/// the row says which. A member moves from here to the bridge only by an argument that the reason
/// no longer holds.
pub const REFUSED: &[Excluded] = &[
    Excluded {
        holder: Holder::Doc,
        members: &["documentFileName"],
        reason: "is the name of a file on the reader's machine, which the confined worker has no \
                 business knowing; the document's own name for itself is this.info.Title",
    },
    Excluded {
        holder: Holder::App,
        members: &["goBack", "goForward"],
        reason: "walks the window's history of views, and this viewer keeps none: a person has no \
                 previous view to go back to either, so there is nothing a script's request could \
                 reach",
    },
];

/// What RFC 0008 section 4.2 admits to Tier 1 and this bridge does not carry, by holder.
///
/// The bridge carries the document's realm and its fields (ADRs 1591, 1602, 1603), the viewer's
/// identity, `util.printd` and `util.printx`, a field's `getArray` and `setFocus` and its text
/// flags' writes (ADR 1615), and `global`, `event.commitKey`, `fieldFull` and `changeEx`,
/// `this.dirty`, `info` and `getOCGs`, `util.printf`, the button captions, `app.alert` and
/// `app.response` (ADRs 1626, 1627), one widget of a field (ADR 1664) and `Field.style` (ADR
/// 1665), a check box's `isBoxChecked` and `checkThisBox` (ADR 1689), `this.getAnnots` and
/// `getAnnot` with the `Annotation` object (ADR 1700), the document's pages, its named
/// destinations, `title`, `calculate` and `app.activeDocs` (ADR 1724), a field's options
/// read from `/Opt` (ADR 1725), the window's `zoom`, `zoomType`, `layout` and `scroll` (ADR
/// 1736), the four members that rewrite a field's `/Opt` (ADR 1737), and the last 21 (ADR 1762):
/// `event.shift`, `modifier`, `keyDown`, `richValue` and `richChange`, a field's `lineWidth`,
/// `textSize` and `textFont`, the page's words, `util`'s `scand`, `crackURL`, `spansToXML`,
/// `xmlToSpans`, `streamFromString`, `stringFromStream` and `iconStreamFromIcon` — the last refused
/// with its reason — `console`'s `show`, `hide` and `clear`, and `OCG.getIntent`.
///
/// Empty: every member section 4.2 admits is carried, or refused by name in [`REFUSED`] or by its
/// own native with a reason. The list stays as the place an admitted member waits, each a property
/// that says so, should a later reading of the reference find one.
pub const NOT_BRIDGED: &[(Holder, &[&str])] = &[];
