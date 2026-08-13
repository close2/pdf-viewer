//! Marked-content sections: §8.11's optional-content visibility and §14.9's accessibility
//! entries.
//!
//! A `BMC`/`BDC` … `EMC` section hangs three independent things off one nesting — whether
//! what follows marks the page, what a reader copies from it, and what a screen reader says
//! about it — and the types here are that nesting's state.

use pdf_syntax::{Dictionary, Object};

use super::Interpreter;
use super::report::Unsupported;

/// One open marked-content section, `BMC`/`BDC` to `EMC`.
///
/// Three independent things hang off the same nesting, which is why this is a struct rather
/// than a counter: §8.11.3.2's optional content decides whether what follows *marks the page*,
/// §14.9.4's replacement text decides what a reader *reads back* from it, and §14.9.3's and
/// §14.9.5's decide what a screen reader *says* about it. A section may be any, all or none.
#[derive(Debug, Clone, Default)]
pub(super) struct Marked {
    /// Whether this section's optional content is turned off.
    pub(super) hides: bool,
    /// Where in the readback this section's text began, for the two rules that need its extent.
    pub(super) starts_at: usize,
    /// §14.9.4's `/ActualText`, which replaces what the section reads back.
    pub(super) actual_text: Option<String>,
    /// §14.9.3's `/Alt` and §14.9.5's `/E`, which replace what it is *spoken* as, and
    /// §14.9.2's `/Lang`, which says in what language.
    ///
    /// `None` where the section states none of the three, which is what keeps an untagged page
    /// from allocating anything.
    pub(super) described: Option<Accessible>,
    /// §14.8.2.2's `/Artifact` tag, with Table 363's property list where the section has one.
    pub(super) artifact: Option<crate::structure::Artifact>,
    /// §14.13.5's `/AF` tag: the files associated with the graphics objects it encloses.
    ///
    /// Empty for every other section, which is all of them in 967 of the 974 corpus documents.
    pub(super) associated: Vec<crate::attachment::Attachment>,
    /// §14.7.5.2's `/MCID`, where this section's property list states one.
    ///
    /// The identifier is what ties a run of page content to a structure element, and therefore
    /// to §14.8.2.5.1's logical content order — which is a different order from the one this
    /// interpreter reads the stream in, and the only reason it is recorded.
    pub(super) mcid: Option<i64>,
    /// Whether this section's tag is §14.8.2.5.3's `ReversedChars`.
    ///
    /// A flag per section rather than one on the interpreter, because the sections nest and
    /// what has to be undone at `EMC` is this one's contribution.
    pub(super) reversed: bool,
}

/// §14.9's three spoken-form entries as one section states them.
#[derive(Debug, Clone, Default)]
pub(super) struct Accessible {
    /// §14.9.3's `/Alt`.
    pub(super) alt: Option<String>,
    /// §14.9.5's `/E`.
    pub(super) expansion: Option<String>,
    /// §14.9.2's `/Lang`, already resolved through a structure element's ancestry.
    pub(super) language: Option<String>,
}

impl Accessible {
    /// `None` where nothing was stated, so that a section with no accessibility entries costs
    /// no allocation and produces no span.
    fn or_nothing(self) -> Option<Self> {
        let stated = self.alt.is_some() || self.expansion.is_some() || self.language.is_some();
        stated.then_some(self)
    }
}

impl Interpreter<'_> {
    /// Whether the content being interpreted right now belongs to a hidden layer.
    ///
    /// What this suppresses is *marking the page*, and nothing else. §8.11.3.1 is explicit
    /// that hiding changes what is drawn and not what the graphics state becomes: colour,
    /// transformation and clipping "shall still be applied", the text position is updated
    /// "even for text wrapped in optional content", and the state after the section is the
    /// same whether it was drawn or not. Suppressing at the point a command enters the
    /// display list is what makes that true by construction rather than by care.
    pub(super) fn is_hidden(&self) -> bool {
        self.hidden > 0
    }

    /// Whether content governed by `oc` is drawn, reporting what cannot be decided.
    ///
    /// `oc` is what a `BDC /OC`'s name finds in the page's `/Properties`, or the `/OC` entry
    /// of an `XObject` or an annotation — **as written**, reference and all. An optional
    /// content group is identified by which object it is (§8.11.2.2), so resolving it before
    /// this point loses the only identity it has.
    pub(super) fn shows_optional_content(&mut self, oc: &Object) -> bool {
        use crate::optional_content::Visibility;

        let Some(optional_content) = &self.optional_content else {
            // §8.11.4.2: with no `/OCProperties`, "a PDF processor shall ignore any optional
            // content structures in the document".
            return true;
        };
        match optional_content.visibility(self.document, oc) {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            Visibility::TooDeep => {
                self.note(Unsupported::OptionalContent {
                    detail: "a /VE visibility expression nested past the interpreter's bound"
                        .to_owned(),
                });
                true
            }
        }
    }

    /// §14.9's four entries for a marked-content sequence, from either place each may live.
    ///
    /// The clause puts `/ActualText` (§14.9.4), `/Alt` (§14.9.3), `/E` (§14.9.5) and `/Lang`
    /// (§14.9.2.1) on a `Span` property list *and* on a structure element, and says the same
    /// thing of both. A sequence carrying an `/MCID` names its element through §14.7.5.4's
    /// parent tree, so both are reachable from a `BDC`; **the property list is asked first for
    /// each entry independently**, because it is the more specific statement — attached to
    /// this sequence rather than to the element the sequence belongs to — and because a file
    /// may put one entry in each place. Falling back per entry rather than per dictionary is
    /// what makes §14.9.3's own example legal: `/Span <</Lang (en-us) /Alt (six-point star)>>`
    /// beside an element that states only the language.
    ///
    /// The element is resolved at most once, and only for a sequence that named one. That
    /// matters: `structure.rs` records that following every entry of the specification's own
    /// first page costs 96 M instructions, so a lookup per entry would pay it four times.
    pub(super) fn accessibility(
        &self,
        resources: &Dictionary,
        operand: Option<&Object>,
    ) -> (Option<String>, Option<Accessible>) {
        let Some(list) = self.property_list(resources, operand) else {
            return (None, None);
        };
        let stated = |key: &str| match self.document.get_key(&list, key) {
            Object::String(bytes) => {
                let text = pdf_syntax::text_string(&bytes);
                (!text.is_empty()).then_some(text)
            }
            _ => None,
        };
        let mut actual_text = stated("ActualText");
        let mut described = Accessible {
            alt: stated("Alt"),
            expansion: stated("E"),
            language: stated("Lang"),
        };

        // The structure element supplies only what the property list did not state, and is
        // read only where one of the four is still missing *and* an `/MCID` names it.
        let missing = actual_text.is_none()
            || described.alt.is_none()
            || described.expansion.is_none()
            || described.language.is_none();
        if missing
            && let Some(mcid) = self.document.get_key(&list, "MCID").as_integer()
            && let Some(element) = self.structure.element(self.document, mcid)
        {
            let document = self.document;
            actual_text = actual_text.or_else(|| crate::structure::actual_text(document, &element));
            described.alt = described
                .alt
                .or_else(|| crate::structure::alternate_description(document, &element));
            described.expansion = described
                .expansion
                .or_else(|| crate::structure::expansion(document, &element));
            // The one of the four that is inherited: §14.9.2.3 has an element take its
            // language from "any parent element that has one".
            described.language = described
                .language
                .or_else(|| crate::structure::language(document, &element));
        }
        (actual_text, described.or_nothing())
    }

    /// The property list a `BDC` operand names, inline or through `/Properties`.
    ///
    /// §14.6.2 gives the operand two forms, and which one a producer may use is decided by the
    /// values: a list of direct objects "may be written inline in the content stream as a
    /// direct object", and one holding an indirect reference "shall be defined as a named
    /// resource in the Properties subdictionary". Both are read here, which is what lets
    /// §14.9.4's `/ActualText` be found wherever a producer put it —
    /// §8.11.3.2's optional content is the one caller that cannot use this, because it needs
    /// the group's *identity* rather than its value.
    pub(super) fn property_list(
        &self,
        resources: &Dictionary,
        operand: Option<&Object>,
    ) -> Option<Dictionary> {
        match operand? {
            Object::Dictionary(list) => Some(list.clone()),
            Object::Name(name) => self
                .resource(resources, "Properties", name.as_str()?)
                .and_then(|list| list.as_dict().cloned()),
            _ => None,
        }
    }
}
