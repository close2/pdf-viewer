//! The files and structure family of the decision table: the rows whose requirement identifier
//! begins `file-structure/`, `implementation-limits/`, `document-requirements/`,
//! `logical-structure/` or `optional-content/`, and the sentences those rows quote.
//! `doc/pdfa-blocking-issues.md` section 6 holds them with the annotations and attachments.
//!
//! Whichever of [`super::REMEDIES`], [`super::CONDITIONAL`], [`super::SHAPES`] and
//! [`super::REFUSED_BY_NAME`] this family has rows for reads them beside every other family's, and
//! the dispatch in [`super`] reads only those four tables, so where a row is written decides
//! nothing about its answer (`doc/adr/2634`).

use super::super::rewrite::Rewrite;
use super::{Answer, Because, Conditional, Loss, Losses, Remedy, Shape};

/// What writing a filter's name out in full asserts (`doc/adr/1952`).
const FILTER_NAMES_REINTERPRET: &str = "a stream in this file names a filter by the abbreviation \
     ISO 32000-2 8.9.7's Table 92 gives it for inline images, which that subclause says shall not \
     be used anywhere else, and the stream now names the filter in full. The standard says \
     nothing of what a reader does with such a name outside an inline image; this reader decodes \
     it as the filter Table 92 names, so the pages were drawn through that filter and still are. \
     The stream's bytes are the producer's, unchanged. The report names every stream rewritten";

/// This family's rows of [`super::REMEDIES`].
pub(super) const REMEDIES: &[Remedy] = &[
    // ISO 19005-2 section 6.1.2, ISO 19005-4 section 6.1.2. Every output of this verb is a new
    // file whose header the serializer writes; `version_for` decides what it states.
    Remedy {
        requirement: "file-structure/file-header",
        answer: Answer::Mechanical(Rewrite::FileHeader),
    },
    // ISO 19005-4 section 6.1.12.
    Remedy {
        requirement: "file-structure/catalog-version-key",
        answer: Answer::Mechanical(Rewrite::CatalogVersion),
    },
    // ISO 19005-2 section 6.1.12, ISO 19005-4 section 6.1.11: a permissions dictionary's keys
    // outside Table 263. §12.8.6 makes each key the name of a permission handler, and a handler
    // the standard does not define is one no conforming processor can consult, so removing the
    // key changes what no reader could see — `super::super::signatures::foreign_handlers`.
    Remedy {
        requirement: "file-structure/permissions-dictionary-keys",
        answer: Answer::Mechanical(Rewrite::ForeignPermissionHandlers),
    },
    // ISO 19005-2 section 6.1.12's second sentence: the digest keys of the DocMDP signature's
    // reference dictionaries. They go with the signature, which `doc/pdf-a-conversion-limits.md`
    // section 3.6 makes an authorised loss — the conversion invalidates the signature whatever
    // is done about the keys, and `super::super::signatures` is the reading.
    Remedy {
        requirement: "file-structure/document-signature-states-no-digest",
        answer: Answer::Loses(Loss::SignatureAssertion, Rewrite::SignatureValueRemoved),
    },
    // ISO 19005-2 section 6.1.3 and ISO 19005-4 section 6.1.3 forbid the trailer's `/Encrypt`;
    // sections 6.1.7.2 and 6.1.6.2 forbid a `Crypt` filter naming anything but `Identity`. One
    // act answers both, because §7.6.2 makes encryption a property of the file: the objects this
    // conversion carries were decrypted when the source was opened, and the output is written
    // from them with no encryption dictionary. `doc/pdf-a-conversion-limits.md` section 3.5 makes
    // it an *Ask* — what the file stops carrying is Table 22's permission flags.
    // Section 5.1 of both parts through ISO 32000-2 section 14.7.5.4: a content item whose holder
    // states no key, a key the parent tree holds no entry under, or an entry naming no element
    // that claims it. The structure tree is the authority — the clause makes the parent tree the
    // means of finding each item's "parent structure elements", so it is derived from their
    // claims — and each lost holder takes a new key and an entry naming its claimant; a producer
    // key that leads back is never changed (`super::super::parent_tree`, `doc/adr/2674`). What
    // cannot be read stays refused by name, in that module and in `super::super::tagged`.
    Remedy {
        requirement: "logical-structure/content-items-find-their-elements-through-the-parent-tree",
        answer: Answer::Mechanical(Rewrite::ParentTreeRebuilt),
    },
    Remedy {
        requirement: "file-structure/no-encryption",
        answer: Answer::Loses(Loss::Encryption, Rewrite::EncryptionRemoved),
    },
    Remedy {
        requirement: "file-structure/crypt-filter-is-identity",
        answer: Answer::Loses(Loss::Encryption, Rewrite::EncryptionRemoved),
    },
    // ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2.
    Remedy {
        requirement: "file-structure/no-lzw-filter",
        answer: Answer::Mechanical(Rewrite::FlateInsteadOfLzw),
    },
    // The same two subclauses' first sentence, at the names this tree's decoder reads: a filter
    // named by its inline-image abbreviation (`doc/adr/1952`). Any other name is the loss
    // `undecodable-stream`, which `CONDITIONAL` names and `super::answer_of` routes to
    // (`super::super::undecodable`, `doc/adr/2689`).
    Remedy {
        requirement: "file-structure/stream-filters-are-standard",
        answer: Answer::Stated(
            None,
            Rewrite::FilterNamesSpelledOut,
            FILTER_NAMES_REINTERPRET,
        ),
    },
    // ISO 19005-2 section 6.11, ISO 19005-4 section 6.12.
    Remedy {
        requirement: "document-requirements/no-requirements-dictionary",
        answer: Answer::Mechanical(Rewrite::RequirementsDictionary),
    },
    // ISO 19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1, first requirement:
    // the program of every font a content stream renders is in the file.
    // `doc/pdf-a-conversion-limits.md` section 4.9 makes embedding a face the default rather than
    // a refusal, `doc/questions/A47` settled it, and the condition A47 attaches — refuse rather
    // than guess where no shipped face covers the characters — is `super::super::fonts`' own gate.
    // ISO 19005-2 section 6.1.7.1, ISO 19005-4 section 6.1.6.1: no stream's data outside the
    // file. The bytes are the caller's to resolve and reach this verb in the plan
    // (`doc/adr/1199`), so the rewrite's feasibility is a question about what was handed over —
    // which is what `Prepared::obstacle` asks, and why a document nobody resolved anything for
    // keeps a refusal naming the switch that would.
    Remedy {
        requirement: "file-structure/no-external-stream-data",
        answer: Answer::Mechanical(Rewrite::ExternalDataEmbedded),
    },
    // ISO 19005-2 section 6.7.2.2, and Level A only. Answered where the file already carries the
    // structure tree the flag is a claim about, and refused with `NO_STRUCTURE_TREE` where it
    // does not — which is `Prepared::obstacle`'s gate rather than a second row.
    Remedy {
        requirement: "logical-structure/mark-info-marked",
        answer: Answer::Mechanical(Rewrite::MarkInfo),
    },
    // ISO 19005-2 section 6.7.4, which binds PDF/A-2a alone: a `/Lang` that is stated is a
    // language identifier. The entry is kept and states §14.9.2.2's unknown language, a loss of
    // the producer's spelling; an operator's `supply` writes their tag through the same two
    // rewrites instead (`doc/adr/2066`, and `doc/adr/2067` for the content stream's half).
    Remedy {
        requirement: super::super::languages::CATALOG_SITE,
        answer: Answer::Loses(Loss::LanguageIdentifier, Rewrite::CatalogLanguageRestated),
    },
    Remedy {
        requirement: super::super::languages::MARKED_SITE,
        answer: Answer::Loses(Loss::LanguageIdentifier, Rewrite::MarkedLanguagesRestated),
    },
    // ISO 19005-4 section 6.1.3's two sentences about the document information dictionary, which
    // part 2 does not state. §14.3.3 deprecates the dictionary in favour of the metadata stream
    // and Table 349's NOTEs name the XMP counterpart of every one of its keys, so the remedy is
    // the standard's own direction of travel rather than a choice: the values move and the
    // dictionary goes. **`Stated` rather than `Mechanical`** because two things an operator has
    // to be told happen — §14.3.4 leaves a value the packet already states as the packet has it,
    // and a custom key Table 349 names nothing for is answered by `unmapped` (`doc/adr/1269`).
    Remedy {
        requirement: "file-structure/document-information-dictionary-needs-piece-info",
        answer: Answer::Stated(
            None,
            Rewrite::InformationMovedIntoThePacket,
            INFORMATION_MOVES_INTO_THE_PACKET,
        ),
    },
    Remedy {
        requirement: "file-structure/document-information-dictionary-holds-only-a-modification-date",
        answer: Answer::Stated(
            None,
            Rewrite::InformationMovedIntoThePacket,
            INFORMATION_MOVES_INTO_THE_PACKET,
        ),
    },
    // ISO 19005-2 section 6.9, ISO 19005-4 section 6.10 — the `/Order` half of that subclause.
    Remedy {
        requirement: "optional-content/order-lists-every-group",
        answer: Answer::Mechanical(Rewrite::OptionalContentOrder),
    },
    // ISO 19005-2 section 6.9, which ISO 19005-4 section 6.10 does not bind: part 4 keeps the key
    // and requires a conforming processor to ignore it. §8.11.4.3's `/AS` is the array a
    // processor sets group states from external factors by, so removing it leaves the document in
    // the state the configuration's own entries put it in and nothing switches afterwards — a
    // loss to authorise rather than a rewrite that costs nothing. `doc/adr/1234`.
    Remedy {
        requirement: "optional-content/no-automatic-states",
        answer: Answer::Loses(Loss::AutomaticStates, Rewrite::AutomaticStatesRemoved),
    },
    // Section 5.1 of both parts, at ISO 32000-2 §7.7.3.2's Table 30 `/Count`: the entry the
    // table defines as the number of descendant leaves, and whose NOTE puts its consistency on the
    // writer, states the leaves this reader walks beneath the node; the pages drawn are the same
    // pages (`doc/adr/2456`).
    Remedy {
        requirement: "file-structure/page-tree-node-count-is-an-integer",
        answer: Answer::Mechanical(Rewrite::PageCountRestated),
    },
    // The same row's integer that disagrees with the leaves beneath its node: the NOTE makes the
    // `/Kids` arrays definitive and the consistency the writer's, so the count written is theirs
    // (`doc/adr/2819`).
    Remedy {
        requirement: "file-structure/page-tree-node-count-counts-its-leaves",
        answer: Answer::Mechanical(Rewrite::PageCountRestated),
    },
    // ISO 19005-2 section 6.1.13's last limit: every page boundary between 3 and 14 400 units.
    // Nine of that subclause's ten limits are `IMPLEMENTATION_LIMITS` refusals and this one is
    // not, because §7.7.3.3's Table 31 makes four of §14.11.2's five boxes **optional** and
    // §14.11.2.1 gives each optional one a default that is another box in the same file. So the
    // removal is the page saying itself the way the table admits rather than an edit to what it
    // says, and whether a reader computes anything different is the second question
    // §14.11.2.1's intersection sentence answers per entry (`doc/adr/1210`). `answer_of` takes
    // that per document, which is why the row is `Mechanical` and `CONDITIONAL` names it.
    Remedy {
        requirement: "implementation-limits/page-boundary-sizes",
        answer: Answer::Mechanical(Rewrite::PageBoundaryRemoved),
    },
];

/// ISO 19005-2 section 6.1.7.2, ISO 19005-4 section 6.1.6.2: the filter row's answer where a name
/// is no abbreviation, so that a stream no reader decodes is left, written as this reader drew it,
/// absent; refused by its reason where a stream's role is undecided (`super::super::undecodable`,
/// `doc/adr/2689`).
pub(super) const WRITTEN_ABSENT: Answer =
    Answer::Loses(Loss::UndecodableStream, Rewrite::UndecodableStreamRemoved);

/// This family's rows of [`super::CONDITIONAL`].
pub(super) const CONDITIONAL: &[(&str, Conditional)] = &[
    (
        "file-structure/no-external-stream-data",
        Conditional::CallerSuppliesTheBytes,
    ),
    (
        "implementation-limits/page-boundary-sizes",
        Conditional::PerDocument(Losses::one(Loss::PageBoundary)),
    ),
    // An abbreviation is spelled out and loses nothing; a name that is no abbreviation leaves a
    // stream no reader of the standard decodes, written as this reader drew it, absent, at the
    // loss `undecodable-stream` (ADR 2490's R07, `doc/adr/2689`).
    (
        "file-structure/stream-filters-are-standard",
        Conditional::PerDocument(Losses::one(Loss::UndecodableStream)),
    ),
];

/// This family's rows of [`super::SHAPES`].
pub(super) const SHAPES: &[(&str, &[Shape])] = &[
    // ISO 19005-2 section 6.1.10, ISO 19005-4 section 6.1.9. Both halves keep the refusal
    // `REFUSED_BY_NAME` states for them — the `/F` entry is inside the producer's content stream —
    // and the split is what a departure narrows on, not what a remedy selects.
    (
        "file-structure/inline-image-filters",
        &[
            Shape {
                name: "LZWDecode",
                answered: false,
            },
            Shape {
                name: "Crypt",
                answered: false,
            },
        ],
    ),
];

/// This family's rows of [`super::REFUSED_BY_NAME`].
pub(super) const REFUSED_BY_NAME: &[(&str, Because)] = &[
    // `doc/pdf-a-conversion-limits.md` section 5.1's refusal, at the three clauses that are the
    // structure tree itself. `CLAUDE.md`'s "authoring content from nothing" is the fence, and
    // ISO 19005-2 section 6.7.1 advises writers against exactly this in its own words.
    (
        "logical-structure/tagged-pdf",
        Because::TheFence(WILL_NOT_INVENT_STRUCTURE),
    ),
    (
        "logical-structure/structure-tree-root",
        Because::TheFence(WILL_NOT_INVENT_STRUCTURE),
    ),
    // Section 5.1 of both parts, at ISO 32000-2 §7.7.3.2's Table 30 `/Kids`: a node stating no
    // children has no tree for a rewrite to state, and the verb refuses it before stage 1
    // (`super::super::page_tree`, `doc/adr/2444`); this is the row's own answer for the census.
    (
        "file-structure/page-tree-node-kids-is-an-array",
        Because::TheFence(A_NODE_WITH_NO_CHILDREN),
    ),
    // The same section through Table 30's `/Parent` row and its `/Kids` row's children: the verb
    // declines such a tree before stage 1 and rebuilds it under `recovered-structure`
    // (`super::super::page_tree`, `doc/adr/2558`), so a source reaching this row has the fault at a node
    // that walk does not follow (`doc/adr/2583`).
    (
        "file-structure/page-tree-root-states-no-parent",
        Because::NotBuiltYet(A_TREE_THE_WALK_DOES_NOT_REACH),
    ),
    (
        "file-structure/page-tree-kids-name-dictionaries",
        Because::NotBuiltYet(A_TREE_THE_WALK_DOES_NOT_REACH),
    ),
    // The same section through the `/Count` row: a source holding none of the pages its root
    // counts is refused by the verb before anything is rebuilt or decided, under every profile
    // (`super::super::refuse_a_document_holding_none_of_its_pages`, `doc/questions/A775`, ADR
    // 2670); this is the row's own answer for the census.
    (
        "file-structure/page-tree-holds-a-page-it-counts",
        Because::TheFence(NONE_OF_ITS_PAGES),
    ),
    // ISO 19005-2 section 6.7.3.4 needs a judgement about what a producer's own structure type
    // *meant*, which `doc/pdf-a-conversion-limits.md` section 5.1 calls an **Ask**: the
    // operator's `role-map` answers it in advance (`doc/adr/1990` section 6).
    (
        "logical-structure/role-map-terminates-at-a-standard-type",
        Because::NotBuiltYet(ROLE_MAP_NEEDS_A_JUDGEMENT),
    ),
    // ISO 19005-2 section 6.1.8 and ISO 19005-4 section 6.1.7. A row of this table rather
    // than a branch of [`decide`] (ADR 0954): a refusal `census` cannot see is a
    // refusal that counts as a gap, and this one is an argument rather than a gap.
    (
        "file-structure/bound-names-are-valid-utf8",
        Because::TheFence(UTF8_NAMES),
    ),
    // ---------------------------------------------------------------------------------------
    // The requirements the census found with no considered answer at all (ADR 0955). Each is
    // one of four things and the sentence says which — a fence nothing closes, a target that
    // conforms where this one cannot, a caller's own refusal, or a rewrite named as owed. The
    // census that found them is `super::super::census`, and `tests/archive_unconsidered.txt` is the
    // ratchet that keeps the list from growing back.
    // ---------------------------------------------------------------------------------------

    // ISO 19005-2 section 6.1.13's ten limits. Part 4 states no implementation-limits subclause
    // at all, which is what makes every one of these a target question rather than a gap.
    (
        "implementation-limits/character-identifiers",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/devicen-colourants",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/graphics-state-nesting",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/integer-values",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/name-lengths",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/real-values",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/string-lengths",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    (
        "implementation-limits/values-written-in-content-streams",
        Because::NotThisTarget(IMPLEMENTATION_LIMITS),
    ),
    // ISO 19005-2 section 6.9, ISO 19005-4 section 6.10: the one of section 3.8's three rules
    // still refused. The `/Order` rule and the `/AS` rule are both rows of `REMEDIES`.
    (
        "optional-content/configuration-names",
        Because::NotBuiltYet(CONFIGURATION_NAME_NOT_WRITTEN),
    ),
];

/// Why a page tree Table 30 rules out keeps its refusal where the conversion's own walk passed it.
const A_TREE_THE_WALK_DOES_NOT_REACH: &str = "this page tree has a root that states /Parent or a \
     Kids entry that names no dictionary, which ISO 32000-2 section 7.7.3.2's Table 30 rules out, \
     at a node written inside the catalog or inside its parent's Kids; this conversion declines \
     such a tree, and rebuilds it from the pages this reader draws where recovered-structure is \
     granted, only at a node written as an object of its own";

/// Why a source holding none of the pages it states is not converted (`doc/questions/A775`).
const NONE_OF_ITS_PAGES: &str = "the source's page tree root counts one page or more and the \
     file holds none of them, so it holds nothing a reader sees to archive; it is refused by the \
     name of the loss page-not-held under every profile, never written as an archive of no page";

/// Why a page tree node with no `/Kids` array is not rewritten (`doc/adr/2444`).
const A_NODE_WITH_NO_CHILDREN: &str = "a page tree node here states no Kids array, so the tree \
     names no child beneath it: the pages a reader draws from such a node - none, or the ones a \
     scan of the file finds - are not the producer's tree, and writing one would be this \
     converter deciding which pages the document has";

/// `doc/pdf-a-conversion-limits.md` section 5.1's sentence, at the clauses that are the tree.
const WILL_NOT_INVENT_STRUCTURE: &str = "this converter will not invent a structure tree. \
     Deciding that this run of glyphs is a heading and that one a table cell, what the reading \
     order of a two-column page is and what an image depicts, is authoring rather than \
     converting — and a wrong reading order is worse than none, because it misleads \
     confidently. ISO 19005-2 section 6.7.1 advises writers against adding structural \
     information not present in the source solely to achieve conformance. A document that \
     already carries a tree converts to PDF/A-2a; this one does not, and PDF/A-2u and PDF/A-2b \
     ask nothing of logical structure";

/// Why an unmapped non-standard structure type is not role-mapped.
const ROLE_MAP_NEEDS_A_JUDGEMENT: &str = "mapping a non-standard structure type to the nearest \
     standard one is a statement about what the producer's own type meant, and this converter \
     does not guess it: Chapter to Sect is a guess that happens to be right and Sidebar to Note \
     is one that may not be. It is a question for the document's owner: a configuration that \
     answers this site with remedy = \"supply\" and a role-map table naming the standard type each \
     of the producer's types means writes those entries into the role map (doc/adr/1990). A type \
     the table does not name, an element in a namespace of its own, or a chain the producer's map \
     already ends elsewhere stays refused";

/// Why a document outside ISO 19005-2's implementation limits is a PDF/A-4 document.
///
/// `doc/pdf-a-conversion-limits.md` section 2.5. Ten requirements share this: every one of them
/// is a limit on what the file may contain rather than on what it may say, so every fix is an
/// edit to the document's own values — and part 4 states no implementation-limits subclause at
/// all, which turns the whole family into a question about the target.
const IMPLEMENTATION_LIMITS: &str = "ISO 19005-2 section 6.1.13 sets a hard limit this document \
     exceeds, and every way of meeting one is an edit to what the document says: re-nesting q \
     and Q, rescaling a page, re-encoding a colour space, renaming an object, or dropping \
     objects until the count falls. ISO 19005-4 states no implementation-limits subclause at all \
     — its section 6.1 runs to 6.1.12 — so a large-format drawing, a deeply nested content \
     stream or a 40-colourant DeviceN space can be PDF/A-4 and cannot be PDF/A-2. Ask for \
     PDF/A-4";

/// What an operator is told when `/Info` moves into the XMP packet.
///
/// Every value written is the producer's and every property it is written under is the one
/// §14.3.3's Table 349 names for that key — so nothing is invented. What an operator has to be
/// told is that a reader's File → Properties panel now reads from somewhere else, and that a key
/// the packet already states keeps the packet's value rather than the dictionary's, which is
/// §14.3.4's own instruction.
const INFORMATION_MOVES_INTO_THE_PACKET: &str = "the document information dictionary is \
     deprecated by §14.3.3 and ISO 19005-4 section 6.1.3 admits it only beside a PieceInfo and \
     holding only a ModDate, so each of its entries is written into the XMP packet under the \
     property Table 349's own NOTE names for it and the dictionary goes. Every value is the \
     producer's. Where the packet already states a property the dictionary also states, the \
     packet's value stands and the dictionary's is dropped, which is §14.3.4's instruction to \
     leave inconsistent values unchanged; a key Table 349 names no counterpart for is the \
     `unmapped` answer's. What a reader sees change is where File → Properties reads from";

/// Why a configuration's `/Name` is not written.
///
/// The `/Order` half of section 3.8's pair is built — `Rewrite::OptionalContentOrder` — and this
/// is the half that is not, because the two are different acts. Completing an array with groups
/// the file already lists invents nothing; a name does not exist anywhere to be found.
const CONFIGURATION_NAME_NOT_WRITTEN: &str = "an optional content configuration here states no \
     Name, or states one another configuration already uses. §8.11.4.3 makes Name a label for a \
     user interface, so no name is in the file to be found, and doc/questions/A48's line forbids \
     this converter filling in an absence on its own authority. A configuration that answers \
     this site with remedy = \"supply\" and a name-template writes the operator's own label, \
     numbered past every name the file uses, into each configuration that states none \
     (doc/adr/1952); a name two configurations share stays refused, because choosing which one \
     keeps it would be renaming the producer's text";

/// The document does not conform and cannot be made to without editing what it says.
const UTF8_NAMES: &str = "ISO 19005 binds these names to valid UTF-8, and renaming one would \
     break every reference that uses it — doc/pdf-a-conversion-limits.md section 4.7 records this \
     row \
     as a refusal rather than a rewrite";
