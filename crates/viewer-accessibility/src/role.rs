//! §14.8.4's forty-one standard structure types, and the eleven only the PDF 1.7 namespace
//! defines, onto AccessKit's roles.
//!
//! # Two vocabularies, and what one says that the other cannot
//!
//! ISO 32000-2 §14.8.4 names forty-one structure types across five tables; `accesskit::Role` has
//! about two hundred, drawn from ARIA, from the `DPub` module and from three desktop platforms'
//! own trees. §14.8.6.1 makes PDF 1.7's namespace the default one, so the eleven types Annex M lists
//! as defined there and not in PDF 2.0's — ISO 32000-1's `Art`, `BlockQuote`, `TOC`, `TOCI`,
//! `Index`, `Private`, `Quote`, `Note`, `Reference`, `BibEntry` and `Code` — are standard too, and
//! are the commonest names in the corpus's tagged documents that once crossed as undefined (ADR
//! 2014). Most of the fifty-two land on one exactly. The rest are recorded here rather than
//! defaulted to [`accesskit::Role::Unknown`], because a mapping that quietly loses a distinction
//! is worse than one that says which distinction it lost:
//!
//! | §14.8.4 | becomes | and what is lost |
//! |---|---|---|
//! | `DocumentFragment` | `Document` | AccessKit has no fragment role. §14.8.4.3's difference is that "the structure of the fragment may be incomplete", which is not something an assistive technology can act on |
//! | `Part` | `Group` | `Role::DocPart` exists and is the `DPub` `doc-part`, "a major thematic section" — which asserts the hierarchy §14.8.4.4 says a `Part` is explicitly *without*: "a grouping of structure elements without consideration for their hierarchy" |
//! | `Title` | `Heading` | no title role; a title is what a person navigating by heading wants to land on. The *level* is not set, because §14.8.4.5 gives a `Title` none |
//! | `FENote` | `DocFootnote` | one PDF type for two `DPub` roles. §14.8.4.5 calls it "footnotes and endnotes" and AccessKit splits them; the footnote is the commoner and the endnote loses its name |
//! | `Sub` | `Group` | no sub-block role anywhere in AccessKit |
//! | `Lbl` | `Label` | inside a list this is the bullet, and `Role::ListMarker` would say so — but §14.8.4.7.2 makes `Lbl` general ("content that distinguishes it from other content inside the same parent element"), and a line number in a `Sub` is not a marker |
//! | `Annot` | `Group` | AccessKit has no role for "content associated with an annotation", and which annotation it is is §14.7.5.3's `/OBJR`, which this program locates but does not classify |
//! | `Form` naming no widget | `Group` | see below |
//! | `RB`, `RP`, `WT`, `WP` | `Label` | AccessKit has `Ruby` and `RubyAnnotation` and nothing for the base text, the punctuation or warichu's halves |
//! | `Warichu` | `Group` | no warichu role |
//! | `LBody` | `GenericContainer` | no list-item-body role; a list item's content belongs to the item, which is what the platform filter does with this one |
//! | `TH` scoped `Both` | `ColumnHeader`, described | AccessKit splits header cells by axis and has no role for a cell that describes both. See below |
//! | `TOC` | `DocToc` | the `DPub` table-of-contents landmark, which a nested `TOC` is too: ISO 32000-1 Table 333 makes the nesting the hierarchy, and AccessKit has no sub-contents role |
//! | `TOCI` | `ListItem` | no contents-entry role; an entry is one member of a list of them |
//! | `Quote`, `Reference` | `Label` | no inline quotation role (`Blockquote` is `BlockQuote`'s, a grouping of paragraphs), and no role for a citation inside the document — `DocNoteRef` and `DocBiblioRef` each name one kind, and a contents entry's `Reference` is neither |
//! | `Note` | `DocFootnote` | as `FENote`: ISO 32000-1 Table 338 makes it explanatory text such as a footnote or an endnote |
//! | `Private` | `GenericContainer`, not spoken | nothing a host is handed: ISO 32000-1 Table 333 says neither it nor any of its descendants is to be interpreted, and `viewer-core`'s walk gathers neither (ADR 2028). Mapped all the same, so that a node from another source is withheld rather than described |
//! | `Figure` | `Image` | `Role::Figure` exists and becomes an AT-SPI `Panel`, which says nothing; a `Figure` is graphical content that §14.8.4.8.5 says "should have either an `Alt` entry or an `ActualText` entry", which is an image with alternative text |
//! | anything else | `Group`, described | §14.8.4.1 requires a tagged document's types to be standard or role mapped to standard ones, so a name arriving here is a defect in the *document*. The name is put in the node's description rather than dropped, with which of `pdf_model::structure::Nonstandard`'s reasons it is: a name its namespace does not define, a foreign namespace's word — a `Table` there is a homonym, not a table — a role map that came back on itself, or no type at all |
//!
//! # Two roles the platform filters out, and the rule that follows
//!
//! `accesskit_consumer`'s `common_filter` — which the AT-SPI adapter uses — removes a
//! [`accesskit::Role::GenericContainer`] or [`accesskit::Role::TextRun`] node from the tree an
//! assistive technology sees and lifts its children into its parent. That is exactly right for
//! `NonStruct`, whose clause says the same thing in its own words: "[a] grouping element having
//! no inherent structural significance … it should not be interpreted or exported to other
//! document formats. Its descendants shall be processed".
//!
//! It is also a way to lose text, because a node's *label* goes with the node. So the mapping
//! takes a second input — whether this element has anything to say — and a type that would
//! otherwise be filtered out becomes a [`accesskit::Role::Label`] when it does.
//!
//! **`Artifact` is the one exception, and it is deliberate.** §14.8.2.2.1 divides a page into the
//! author's real content and "[a]ll other content … whether generated by the PDF writer in the
//! course of pagination, layout, or other mechanical processes or introduced by the document
//! author for decoration". A running head read aloud on every page is what tagging exists to
//! prevent, so an artifact's text is dropped rather than promoted.
//!
//! # A `Form` is a control, and §14.8.4.7.2 says which one
//!
//! Table 368 makes the `Form` structure type one that "[e]ncloses a PDF widget annotation and
//! associated content, if any" — Errata Collection 3's Issue #437, which strikes the *association*
//! wording and puts the annotation where the content was — and requires one per widget: "[i]n a
//! tagged PDF, Form shall be used for each PDF widget annotation that belongs to the real content
//! of the document." So it is **one widget annotation**, which is why [`Role::Form`] — a container
//! *of* fields — has never been the answer, and why [`Role::Group`] was not either: a person is
//! told there is something on the page and not that it is a check box.
//!
//! §12.7.5's four field types are what the platform has words for, and
//! [`viewer_core::AccessibilityNode::control`] carries them here through §14.7.5.3's object
//! reference. All 272 of the corpus's `Form` elements name a widget whose field type is readable
//! (`pdf-model --example element_bounds_census`), which is what makes this worth the wire field.
//!
//! | §12.7.5 | becomes |
//! |---|---|
//! | a push button, §12.7.5.2.2 | [`Role::Button`] |
//! | a check box, §12.7.5.2.3 | [`Role::CheckBox`], with `toggled` set from the field's value |
//! | a radio button, §12.7.5.2.4 | [`Role::RadioButton`], likewise |
//! | a text field, §12.7.5.3 | [`Role::PasswordInput`] under Table 231 bit 14, else [`Role::MultilineTextInput`] under bit 13, else [`Role::TextInput`] |
//! | a choice field, §12.7.5.4 | [`Role::EditableComboBox`] under Table 233 bits 18 and 19, [`Role::ComboBox`] under bit 18, else [`Role::ListBox`] |
//! | a signature field, §12.7.5.5 | [`Role::Group`], described — see below |
//! | a field stating no `/FT` | [`Role::Group`], described |
//!
//! **Password before multiline**, because the two flags say different kinds of thing and only one
//! of them is about secrecy: Table 231 bit 14 makes the field "intended for entering a secure
//! password that should not be echoed visibly to the screen", and a control that echoed a password
//! because the file also set bit 13 would be the one mistake here that cannot be taken back. A file
//! setting both has said something Table 231 does not contemplate, and this is the safe reading of
//! it rather than a claim about which bit wins.
//!
//! **A signature field keeps [`Role::Group`] and says so.** §12.7.5.5 makes it "a form field that
//! contains a digital signature", and neither AccessKit nor AT-SPI has a role for one; the nearest
//! candidates — a button, a text input — would each assert something about what a person may do
//! with it that this program does not implement. So the loss is named in the description, the way
//! every other loss in this module is.
//!
//! # A header cell's axis, and the one value the platform has no word for
//!
//! §14.8.4.8.3's `TH` is a cell "describing one or more rows, columns or rows and columns of the
//! table", and Table 384's `/Scope` says which of the three — stated by the document, or assumed
//! by §14.8.5.7 from the cell's place in the grid. `viewer_core::AccessibilityNode::header_scope`
//! carries the answer here because only the reading side can work it out.
//!
//! Two of the three land exactly: `Row` is [`accesskit::Role::RowHeader`] and `Column` is
//! [`accesskit::Role::ColumnHeader`], which `accesskit_atspi_common` puts on the bus as AT-SPI's
//! `RowHeader` and `ColumnHeader`. **`Both` has no role in either vocabulary, and what is done
//! with it is a choice rather than a derivation**: the node keeps `ColumnHeader` and says in its
//! description that the document scopes it to both axes. The alternative — a plain `Cell` — loses
//! the fact that it is a header at all, which is the more expensive of the two losses; and
//! inventing an axis the document did not state is what this table exists to avoid.
//!
//! **A `TH` whose axis is unknown is a third case and is not folded into the second.** A document
//! that puts a header cell outside a `TR` leaves §14.8.5.7's assumption nothing to work from, so
//! there is no scope, and the description says the axis could not be determined instead of naming
//! one. That is the same rule the untagged page follows one file over.
//!
//! **What does *not* cross is the cell's coordinates**, and the reason is worth recording because
//! it is about this platform rather than about the standard: `accesskit_atspi_common` implements
//! `Accessible`, `Action`, `Component`, `Hyperlink`, `Selection`, `Text` and `Value`, and **not**
//! `org.a11y.atspi.Table` or `TableCell`. A row and column index set on a node would therefore
//! reach AccessKit and stop there, which is the shape of capability `doc/habits.md` warns about.
//! `doc/todo/31` records it as owed to the platform rather than to this crate.

use accesskit::Role;
use pdf_model::form::Control;
use pdf_model::structure::{HeaderScope, Nonstandard, StandardType};

/// What one structure type becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    /// The platform role.
    pub role: Role,
    /// §14.8.4.5's `Hn` level, where the type states one.
    ///
    /// > With n being a sequence of digits representing an unsigned integer greater than or equal
    /// > to 1
    ///
    /// so the level is part of the *name* and `H7` is as legal as `H1`. `None` for `H`, whose
    /// level "is not indicated through this structure element, but instead is derived from the
    /// nesting of the logical structure" — a derivation this program does not do, and a level
    /// invented here would be a claim about the document rather than a reading of it.
    pub level: Option<u32>,
    /// The type the document wrote, where §14.8.4 does not define it, and why it is not one.
    ///
    /// `Some` is a finding about the *document*: §14.8.4.1 requires "[a]ll structure elements
    /// occurring within a tagged PDF document" to have "a type matching one of those defined as a
    /// Standard Structure Type, or a role map providing a mapping from the non-standard type to a
    /// Standard Structure Type", and the role map has already been applied by the time a name
    /// reaches here. It is carried into the node's description rather than dropped, because a
    /// person whose reader says "group" wants to know that the file, not the reader, is why.
    pub unmapped: Option<(String, Nonstandard)>,
    /// Whether a label may be put on the node.
    ///
    /// `false` for `Artifact`: §14.8.2.2 makes an artifact content that is on the page and is not
    /// the document's, and speaking it is what tagging exists to avoid. And for the PDF 1.7
    /// namespace's `Private`, whose ISO 32000-1 Table 333 entry says neither it nor any of its
    /// descendants is to be interpreted or exported (ADR 2014).
    pub speaks: bool,
    /// What the platform's vocabulary could not carry, in words a person is meant to hear.
    ///
    /// `Some` only where the mapping *lost* something a document stated — a `TH` scoped to both
    /// axes, or one whose axis could not be determined — and it becomes the node's description
    /// rather than its name, so it is advisory rather than read in place of the cell's content.
    pub note: Option<&'static str>,
    /// Whether §12.7.5.2's toggling button is on, for a `Form` naming one and nothing else.
    ///
    /// A check box announced without its state says there is a box and not whether it is ticked,
    /// which is the whole of what the control means. §12.7.5.2.3 makes the field one that "toggles
    /// between two states, on and off" and Table 226's `/V` is which — already resolved through
    /// this view's own state by `pdf_model::form`, so a box a person has just clicked answers with
    /// what they did.
    ///
    /// `None` for every control that does not toggle, which is what
    /// [`accesskit::Toggled`] has no value for: a text field is neither on nor off.
    pub toggled: Option<bool>,
}

/// Maps one element's structure type onto the platform's vocabulary.
///
/// `standard` is what the type means in the namespace it ends in, which
/// [`viewer_core::AccessibilityNode::standard`] answers with; `name` is the name the role map
/// ended at, read for nothing but the description of a type that is not standard. **The name is
/// never read as a type here**: a foreign namespace's `Table` is not §14.8.4's, and only the
/// reading side, which holds the namespace, can say so (ADR 2028).
///
/// `speaking` is whether the element has anything of its own to say, which decides one thing: a
/// type that would otherwise become a role the platform filters out of the tree keeps its text by
/// becoming a [`Role::Label`] instead. See this module's own documentation.
///
/// `titled` is whether that name is Table 355's `/T` rather than the element's own text
/// ([`viewer_core::AccessibilityNode::titled`]), which decides one thing too: a `Sect` with a title
/// is a [`Role::Region`], a section a person can be taken to by name (ADR 1405).
///
/// `scope` is Table 384's axis for a `TH`, which
/// [`viewer_core::AccessibilityNode::header_scope`] answers with and which decides between two
/// roles the platform keeps apart. It is ignored for every other type, because the entry "shall
/// only have an effect for structure elements of type of TH".
///
/// `control` is §12.7.5's field type for the widget annotation behind §14.7.5.3's object
/// reference, which [`viewer_core::AccessibilityNode::control`] answers with. It is read for
/// `Form` and nothing else, because §14.8.4.7.2 makes that the type a widget belongs to — see this
/// module's own documentation.
#[must_use]
pub fn map(
    standard: Result<&StandardType, Nonstandard>,
    name: &str,
    speaking: bool,
    titled: bool,
    scope: Option<HeaderScope>,
    control: Option<&Control>,
) -> Mapping {
    let standard = match standard {
        Ok(standard) => standard,
        Err(reason) => {
            return Mapping {
                role: Role::Group,
                level: None,
                unmapped: Some((name.to_owned(), reason)),
                speaks: true,
                note: None,
                toggled: None,
            };
        }
    };
    #[expect(
        clippy::match_same_arms,
        reason = "the arms are §14.8.4's own five tables in the standard's order, and merging \
                  the ones that happen to land on the same role would hide which table a type \
                  came from — which is the thing this mapping has to be readable against"
    )]
    let (mapped, level) = match standard {
        // Table 364, document level.
        StandardType::Document | StandardType::DocumentFragment => (Role::Document, None),
        // Table 365, grouping level.
        StandardType::Part => (Role::Group, None),
        // A titled `Sect` is ARIA's `region`, which AT-SPI carries as a landmark a person can be
        // taken to by name: §14.8.4.4 makes a `Sect` a grouping "with consideration for their
        // hierarchy", and its `/T` is the name that hierarchy is navigated by (ADR 1405). A `Div`
        // stays a section whatever it states, because the same table makes it "orthogonal to the
        // semantic structure".
        StandardType::Section if titled => (Role::Region, None),
        StandardType::Section | StandardType::Division => (Role::Section, None),
        // "content that is distinct from other content within its parent structure element",
        // whose examples are callouts, sidebars and commentary — which is ARIA's `complementary`.
        StandardType::Aside => (Role::Complementary, None),
        StandardType::NonStructural => (Role::GenericContainer, None),
        // Table 366, block level.
        StandardType::Paragraph => (Role::Paragraph, None),
        StandardType::Heading(level) => (Role::Heading, Some(*level)),
        StandardType::UnnumberedHeading | StandardType::Title => (Role::Heading, None),
        StandardType::FootnoteOrEndnote => (Role::DocFootnote, None),
        // Table 367, sub-block level.
        StandardType::SubBlock => (Role::Group, None),
        // Table 368, inline level.
        StandardType::Label => (Role::Label, None),
        StandardType::Span => (Role::Label, None),
        StandardType::Emphasis => (Role::Emphasis, None),
        StandardType::Strong => (Role::Strong, None),
        StandardType::Link => (Role::Link, None),
        StandardType::Annotation => (Role::Group, None),
        StandardType::Form => (form_role(control), None),
        // Table 369, ruby and warichu.
        StandardType::Ruby => (Role::Ruby, None),
        StandardType::RubyText => (Role::RubyAnnotation, None),
        StandardType::RubyBase
        | StandardType::RubyPunctuation
        | StandardType::WarichuText
        | StandardType::WarichuPunctuation => (Role::Label, None),
        StandardType::Warichu => (Role::Group, None),
        // Table 370, lists.
        StandardType::List => (Role::List, None),
        StandardType::ListItem => (Role::ListItem, None),
        StandardType::ListItemBody => (Role::GenericContainer, None),
        // Table 371, tables.
        StandardType::Table => (Role::Table, None),
        StandardType::TableRow => (Role::Row, None),
        StandardType::TableHeader => (header_role(scope), None),
        StandardType::TableData => (Role::Cell, None),
        StandardType::TableHead | StandardType::TableBody | StandardType::TableFoot => {
            (Role::RowGroup, None)
        }
        // Tables 372 to 375.
        StandardType::Caption => (Role::Caption, None),
        StandardType::Figure => (Role::Image, None),
        StandardType::Formula => (Role::Math, None),
        StandardType::Artifact => (Role::GenericContainer, None),
        // The eleven the PDF 1.7 namespace alone defines: ISO 32000-1's Table 333, grouping
        // level, then its Table 338, inline level (ADR 2014).
        StandardType::Article => (Role::Article, None),
        StandardType::BlockQuote => (Role::Blockquote, None),
        StandardType::TableOfContents => (Role::DocToc, None),
        StandardType::TableOfContentsItem => (Role::ListItem, None),
        StandardType::Index => (Role::DocIndex, None),
        StandardType::Private => (Role::GenericContainer, None),
        StandardType::Quote | StandardType::Reference => (Role::Label, None),
        StandardType::Note => (Role::DocFootnote, None),
        StandardType::BibliographyEntry => (Role::DocBiblioEntry, None),
        StandardType::Code => (Role::Code, None),
    };
    // `Private` is not spoken either, for ISO 32000-1 Table 333's reason, which is `Artifact`'s in
    // other words: neither the element nor any of its descendants is to be interpreted or exported.
    let speaks = !matches!(standard, StandardType::Artifact | StandardType::Private);
    Mapping {
        // A node the platform would filter out takes its label with it, so a type that has
        // something to say may not have one. `Artifact` is not promoted, because its text is
        // what §14.8.2.2 says is not the document's content.
        role: if speaking && speaks && filtered_out(mapped) {
            Role::Label
        } else {
            mapped
        },
        level,
        unmapped: None,
        speaks,
        note: header_note(standard, scope).or_else(|| form_note(standard, control)),
        toggled: form_toggled(standard, control),
    }
}

/// Which of the platform's controls §14.8.4.7.2's `Form` becomes.
///
/// The table in this module's documentation, in code. Two of the arms are decisions rather than
/// lookups and both are argued there: password before multiline, and a signature field keeping
/// [`Role::Group`].
fn form_role(control: Option<&Control>) -> Role {
    match control {
        // A `Form` this program could not follow to a widget, and §12.7.5.5's signature, which
        // neither vocabulary has a word for. `form_note` says which of the three happened.
        None | Some(Control::Signature | Control::Unstated) => Role::Group,
        Some(Control::PushButton) => Role::Button,
        Some(Control::CheckBox { .. }) => Role::CheckBox,
        Some(Control::RadioButton { .. }) => Role::RadioButton,
        Some(Control::Text(text)) => {
            if text.password {
                Role::PasswordInput
            } else if text.multiline {
                Role::MultilineTextInput
            } else {
                Role::TextInput
            }
        }
        // Table 233 bit 18 decides list against combo and bit 19 whether the combo may be typed
        // into. The table restricts the second to the first — "[t]his flag shall be used only if
        // the Combo flag is set" — and `pdf_model::form` carries a file that breaks that rule
        // rather than correcting it, so the list box arm ignores it here too.
        Some(Control::Choice(choice)) => match (choice.combo, choice.editable) {
            (true, true) => Role::EditableComboBox,
            (true, false) => Role::ComboBox,
            (false, _) => Role::ListBox,
        },
    }
}

/// Whether §12.7.5.2's toggling button is on, for the two field types that have a state.
fn form_toggled(standard: &StandardType, control: Option<&Control>) -> Option<bool> {
    if *standard != StandardType::Form {
        return None;
    }
    match control {
        Some(Control::CheckBox { on } | Control::RadioButton { on, .. }) => Some(*on),
        _ => None,
    }
}

/// What a `Form` element's node says where the platform has no control to give it.
fn form_note(standard: &StandardType, control: Option<&Control>) -> Option<&'static str> {
    if *standard != StandardType::Form {
        return None;
    }
    match control {
        None => Some(
            "this document marks a Form structure element (ISO 32000-2 §14.8.4.7.2), which is one \
             widget annotation, but names no widget of a form field on this page, so which \
             control it is is not known",
        ),
        Some(Control::Signature) => Some(
            "this is a signature field (ISO 32000-2 §12.7.5.5); this platform has a role for no \
             such control, so it is given as a group",
        ),
        Some(Control::Unstated) => Some(
            "this form field states no field type (ISO 32000-2 Table 226 makes FT required for a \
             terminal field), so which control it is is not known",
        ),
        Some(
            Control::PushButton
            | Control::CheckBox { .. }
            | Control::RadioButton { .. }
            | Control::Text(_)
            | Control::Choice(_),
        ) => None,
    }
}

/// Which header role a `TH`'s axis becomes.
///
/// `Both` and an axis this reader could not determine both keep [`Role::ColumnHeader`], and
/// [`header_note`] is what says which of the two happened. Neither is silent and neither invents
/// an axis: see this module's own documentation for why the loss falls this way.
fn header_role(scope: Option<HeaderScope>) -> Role {
    match scope {
        Some(HeaderScope::Row) => Role::RowHeader,
        Some(HeaderScope::Column | HeaderScope::Both) | None => Role::ColumnHeader,
    }
}

/// What a header cell's node says about an axis the platform could not carry.
fn header_note(standard: &StandardType, scope: Option<HeaderScope>) -> Option<&'static str> {
    if *standard != StandardType::TableHeader {
        return None;
    }
    match scope {
        Some(HeaderScope::Both) => Some(
            "this header cell describes both its row and its column (ISO 32000-2 Table 384, \
             Scope Both); this platform has a role for each axis and none for both, so it is \
             given as a column header",
        ),
        None => Some(
            "this document states a table header cell outside a table row (ISO 32000-2 \
             §14.8.4.8.3), so which of the table's axes it describes is not known",
        ),
        Some(HeaderScope::Row | HeaderScope::Column) => None,
    }
}

/// Whether the platform adapter drops a node of this role and lifts its children.
///
/// `accesskit_consumer::common_filter`, which every AccessKit platform adapter applies, answers
/// `ExcludeNode` for exactly these two. Stated here as a fact about the library this crate
/// depends on, and asserted against it by `a_filtered_role_never_carries_text`.
fn filtered_out(role: Role) -> bool {
    matches!(role, Role::GenericContainer | Role::TextRun)
}

#[cfg(test)]
mod tests {
    use super::Mapping;
    use accesskit::Role;
    use pdf_model::form::Control;
    use pdf_model::structure::{HeaderScope, Nonstandard, StandardType};

    /// [`super::map`] for a name stating no namespace, read as the default one's — the reading
    /// `pdf_model::structure::Tree::mapped_role` gives such an element whose role map is not
    /// circular.
    fn map(
        name: &str,
        speaking: bool,
        titled: bool,
        scope: Option<HeaderScope>,
        control: Option<&Control>,
    ) -> Mapping {
        let standard = StandardType::read(name);
        super::map(
            standard.as_ref().ok_or(Nonstandard::Undefined),
            name,
            speaking,
            titled,
            scope,
            control,
        )
    }

    /// Every one of §14.8.4's forty-one types maps, and none of them lands on `Unknown`.
    ///
    /// The list is the standard's five tables, written out: 2 document level (Table 364), 5
    /// grouping (365), 5 block (366) — `Hn` counted once — 1 sub-block (367), 7 general inline
    /// (368), 7 ruby and warichu (369), 3 list (370), 7 table (371), and one each for Tables 372
    /// to 375. Forty-one, which is what §14.8.4 defines.
    #[test]
    fn every_standard_structure_type_maps_to_something_that_is_not_unknown() {
        const TYPES: [&str; 41] = [
            "Document",
            "DocumentFragment",
            "Part",
            "Sect",
            "Div",
            "Aside",
            "NonStruct",
            "P",
            "H1",
            "H",
            "Title",
            "FENote",
            "Sub",
            "Lbl",
            "Span",
            "Em",
            "Strong",
            "Link",
            "Annot",
            "Form",
            "Ruby",
            "RB",
            "RT",
            "RP",
            "Warichu",
            "WT",
            "WP",
            "L",
            "LI",
            "LBody",
            "Table",
            "TR",
            "TH",
            "TD",
            "THead",
            "TBody",
            "TFoot",
            "Caption",
            "Figure",
            "Formula",
            "Artifact",
        ];
        for name in TYPES {
            assert!(
                StandardType::read(name).is_some(),
                "{name} is one of §14.8.4's own names"
            );
            let mapping = map(name, false, false, None, None);
            assert_eq!(mapping.unmapped, None, "{name} is standard");
            assert_ne!(
                mapping.role,
                Role::Unknown,
                "{name} landed on Unknown, which is the default this mapping exists to avoid"
            );
        }
    }

    /// The eleven types Annex M lists as the PDF 1.7 namespace's alone are standard too, because
    /// §14.8.6.1 makes that namespace the default — so each maps, none is described as undefined,
    /// and `Private` is withheld as `Artifact` is (ADR 2014).
    #[test]
    fn every_type_only_the_pdf_1_7_namespace_defines_maps() {
        let expected = [
            ("Art", Role::Article),
            ("BlockQuote", Role::Blockquote),
            ("TOC", Role::DocToc),
            ("TOCI", Role::ListItem),
            ("Index", Role::DocIndex),
            ("Private", Role::GenericContainer),
            ("Quote", Role::Label),
            ("Note", Role::DocFootnote),
            ("Reference", Role::Label),
            ("BibEntry", Role::DocBiblioEntry),
            ("Code", Role::Code),
        ];
        for (name, role) in expected {
            let mapping = map(name, true, false, None, None);
            assert_eq!(mapping.unmapped, None, "{name} is standard");
            assert_eq!(mapping.role, role, "{name}");
            assert_eq!(mapping.speaks, name != "Private", "{name}");
        }
    }

    /// §14.8.4.5's `Hn` carries its level, and `H` deliberately does not.
    #[test]
    fn a_numbered_heading_carries_its_level_and_an_unnumbered_one_does_not() {
        assert_eq!(map("H1", true, false, None, None).level, Some(1));
        assert_eq!(map("H6", true, false, None, None).level, Some(6));
        // "with n being a sequence of digits representing an unsigned integer greater than or
        // equal to 1" — so a level nobody enumerated is still a level.
        assert_eq!(map("H17", true, false, None, None).level, Some(17));
        assert_eq!(map("H", true, false, None, None).role, Role::Heading);
        assert_eq!(map("H", true, false, None, None).level, None);
    }

    /// A role the platform filters out never carries the element's text away with it.
    #[test]
    fn a_filtered_role_never_carries_text() {
        // `NonStruct` is the type whose clause asks to be ignored, and the mapping obeys that
        // where there is nothing to lose.
        assert_eq!(
            map("NonStruct", false, false, None, None).role,
            Role::GenericContainer
        );
        assert_eq!(
            map("LBody", false, false, None, None).role,
            Role::GenericContainer
        );
        // And keeps the node where there is.
        assert_eq!(map("NonStruct", true, false, None, None).role, Role::Label);
        assert_eq!(map("LBody", true, false, None, None).role, Role::Label);
    }

    /// §14.8.2.2's artifact keeps its filtered role and is never spoken.
    #[test]
    fn an_artifact_is_not_spoken_even_when_it_has_text() {
        let Mapping { role, speaks, .. } = map("Artifact", true, false, None, None);
        assert_eq!(role, Role::GenericContainer);
        assert!(
            !speaks,
            "§14.8.2.2 makes an artifact content that is not the document's"
        );
    }

    /// The control is read for `Form` and for no other type.
    ///
    /// §14.8.4.7.2 makes `Form` the type a widget annotation belongs to, and Table 368 forbids the
    /// neighbouring `Annot` the same job: "Annot shall not be used for link annotations … or
    /// widget annotations (see the Form structure element)". A file that puts an object reference
    /// on some other element has said something the standard does not describe, and the mapping
    /// does not act on it.
    #[test]
    fn a_control_changes_a_form_and_nothing_else() {
        let control = Control::CheckBox { on: true };
        assert_eq!(
            map("Form", false, false, None, Some(&control)).role,
            Role::CheckBox
        );
        assert_eq!(
            map("Form", false, false, None, Some(&control)).toggled,
            Some(true)
        );
        for name in ["Annot", "P", "Figure", "Link"] {
            let with = map(name, false, false, None, Some(&control));
            assert_eq!(
                with.role,
                map(name, false, false, None, None).role,
                "{name} is not §14.8.4.7.2's widget type"
            );
            assert_eq!(with.toggled, None, "{name}");
        }
    }

    /// A type §14.8.4 does not define is named rather than defaulted.
    ///
    /// The role map has already been applied by the time a name reaches this function, so a name
    /// arriving here is §14.8.4.1's requirement unmet by the document.
    #[test]
    fn a_type_outside_the_standard_set_is_carried_rather_than_dropped() {
        let mapping = map("Advertising", true, false, None, None);
        assert_eq!(mapping.role, Role::Group);
        assert_eq!(
            mapping.unmapped,
            Some(("Advertising".to_owned(), Nonstandard::Undefined))
        );
    }

    /// A foreign namespace's `Table` is mapped as what the reading side resolved — no standard
    /// type — and never as §14.8.4's table because the string matches (ADR 2028).
    #[test]
    fn a_homonym_is_what_the_tree_resolved_and_not_what_its_name_reads_as() {
        let mapping = super::map(Err(Nonstandard::Foreign), "Table", false, false, None, None);
        assert_eq!(mapping.role, Role::Group, "not Role::Table");
        assert_eq!(
            mapping.unmapped,
            Some(("Table".to_owned(), Nonstandard::Foreign))
        );
    }
}
