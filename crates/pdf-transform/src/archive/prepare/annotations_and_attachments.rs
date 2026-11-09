//! The annotations and attachments family of the preparations: what the rows of
//! `decision/annotations_and_attachments` need worked out from the document before they can be
//! answered.
//!
//! An annotation's appearance written or restated, a forbidden or hidden annotation's removal,
//! a widget's appearance states, an embedded file's outcome and the XFA resource kept beside a
//! form are one family because each is an edit to what a page's `/Annots` or the document's name
//! tree holds, and each reads the same annotation and field dictionaries. The family's headings
//! are section 6 of `doc/pdfa-blocking-issues.md`.

use super::super::ArchivePlan;
use super::super::decision::{Because, Loss};
use super::super::preserve::Composed;
use super::super::signatures::Site;
use super::super::sites;
use super::{
    ATTACHMENT_REQUIREMENTS, ForbiddenAnnotations, Fresh, NO_SPARE_OBJECT, NOT_A_PACKET,
    NOT_ASKED_FOR, PACKET_REQUIREMENTS, RemovedAnnotation, Spare,
};
use crate::json::Value;
use pdf_archive::Target;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId, Stream};
use std::collections::{BTreeMap, BTreeSet};

/// Why an annotation's appearance could not be constructed.
const APPEARANCE_NOT_DERIVABLE: &str = "this document holds an annotation with no appearance \
     dictionary whose own subtype clause states no artwork for it — a stamp's legend, a caret, an \
     unapplied redaction, a printer's mark, a trap network, a movie's poster, a watermark, 3D or \
     rich media artwork, or an annotation stating no Subtype or no readable Rect at all. \
     ISO 32000-2 Table 166 requires the dictionary and the subtype clause is what would say what \
     goes in it, so constructing one here would be putting a mark on the page the document never \
     described";

/// Why a partly derivable appearance is not written either.
const APPEARANCE_INCOMPLETE: &str = "this document holds an annotation whose appearance this \
     program can construct only in part — a border style stating no highlight colour, a caption \
     whose room the entry does not give, a field value that would not lay out. A partial \
     rendering written into an archive is not the appearance the clauses state, and a conforming \
     reader would then have nothing else to draw from, so it is refused rather than frozen";

/// Why a check box's or a radio button's widget is not given a constructed appearance.
///
/// A push button's is (`doc/adr/2751`): §12.7.5.2.2 gives it no value, so the one state its
/// subdictionary holds names nothing the field says.
const BUTTON_APPEARANCE_STATES: &str = "this document holds a widget of a check box or radio \
     button field with no appearance dictionary, and §12.7.5.2.3 makes such a widget's N entry a \
     subdictionary of one appearance per state, each state's name being a value of the field. \
     Which states the button has, and what each looks like, is what an absent appearance \
     dictionary does not say, and naming them would be inventing the control's own vocabulary";

/// Why an annotation the page states inline is not given one.
const APPEARANCE_ON_A_DIRECT_ANNOTATION: &str = "this document writes an annotation directly into \
     a page's Annots array rather than as an object of its own, and this verb rewrites objects — \
     so there is nowhere to put the appearance stream the annotation would name";

/// How far up a form field's `/Parent` chain the inheritable `/FT` is looked for.
///
/// `pdf_syntax::Limits::DEFAULT`'s `max_depth`, which is the depth the parser admitted the
/// tree at: a `/Parent` chain longer than that is one no field in the document was read through.
const MAX_FIELD_DEPTH: usize = 256;

/// The appearances a conversion is in a position to construct.
///
/// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 require an appearance dictionary of
/// every annotation but three subtypes and the degenerate rectangle, and
/// `doc/pdf-a-conversion-limits.md` section 4.4 records why constructing one is not invention:
/// §12.5.5 and §12.7.4.3 make the appearance the standard's own construction out of entries the
/// annotation already states. `doc/questions/A21` allows it on one condition, which is
/// [`Self::constructed`].
#[derive(Debug)]
pub(in crate::archive) struct Appearances {
    /// The `/AP` `/N` object each annotation is to name, in the *source's* numbering.
    pub(in crate::archive) at: BTreeMap<ObjectId, ObjectId>,
    /// The form `XObject`s this conversion adds.
    pub(in crate::archive) written: Vec<(ObjectId, Object)>,
    /// The push buttons among [`Self::at`], whose constructed stream is written as the one entry
    /// of a subdictionary under `sites::PUSH_BUTTON_STATE`, with the `/AS` that
    /// selects it.
    ///
    /// ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 make a button widget's `/N` a
    /// subdictionary, `TechNote 0010`'s A012 a push button's included; §12.7.5.2.2 gives a push
    /// button no value, so the state's name is the one `doc/adr/2319` already writes for a push
    /// button's single stream, and it decides nothing a reader draws (`doc/adr/2751`).
    pub(in crate::archive) push_buttons: BTreeSet<ObjectId>,
    /// Every appearance written, for the report.
    pub(in crate::archive) constructed: Vec<WrittenAppearance>,
    /// How many places the requirement failed at that section 6.3.1's removal takes off the page.
    ///
    /// **Not a gap and not a construction: a place that will not exist.** ISO 19005-2 section
    /// 6.3.3 and ISO 19005-4 section 6.3.3 ask an appearance dictionary of the annotations a
    /// *conforming file* holds, and section 6.3.1 forbids some subtypes outright — so an
    /// annotation the removal takes out of its page's `/Annots` is one the output does not hold
    /// and the appearance requirement has no place on. Counting them is what lets
    /// `super::super::decision::answer_of` tell a document whose every failing place goes with the
    /// removal from one that failed nowhere this preparation could see.
    pub(in crate::archive) removed: usize,
}

/// One appearance this conversion constructed, as the report names it.
///
/// `doc/questions/A21`'s condition in one sentence: *report every appearance written, so the
/// difference between the producer's file and ours is visible in the report rather than only in
/// the bytes.* A count would not do it — which annotation on which page now draws this program's
/// marks is the thing a user has to be able to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenAppearance {
    /// The zero-based page the annotation is on.
    pub page: usize,
    /// Its `/Subtype`, or `an annotation stating no subtype` where it states none.
    pub subtype: String,
}

impl WrittenAppearance {
    /// One constructed appearance as JSON.
    pub(in crate::archive) fn to_json(&self) -> Value {
        Value::Object(vec![
            ("page".to_owned(), Value::count(self.page)),
            ("subtype".to_owned(), Value::text(self.subtype.clone())),
        ])
    }
}

/// Constructs the appearance stream every annotation the requirement named would be given.
///
/// **Nothing is judged here either.** `pdf_archive::annotations_without_an_appearance` is the same
/// reading the two requirement rows are, and `pdf_model::appearance` is the construction the
/// viewer already draws from the annotation's own entries. What this decides is whether the
/// construction may be *written down*, and it refuses four cases rather than writing something
/// weaker than the clauses state.
///
/// **`forbidden` narrows the population to the annotations the output will still hold**, which is
/// what the clause asks about: ISO 19005-2 section 6.3.3 and ISO 19005-4 section 6.3.3 bind the
/// annotations of a conforming *file*, and section 6.3.1's removal takes some of the source's out
/// of it. Constructing an appearance for one of those would be drawing a picture for an
/// annotation this conversion is about to delete — and refusing the document because no such
/// picture can be drawn, which is what happened until ADR 1105, refuses it over a place the
/// output does not have. `None` where the removal was not prepared at all, which is every
/// document whose section 6.3.1 row did not fail (ADR 1099).
pub(super) fn prepare_appearances(
    document: &Document,
    target: Target,
    forbidden: Option<&ForbiddenAnnotations>,
    spare: &mut Spare,
) -> Result<Appearances, Because> {
    let mut at = BTreeMap::new();
    let mut written = Vec::new();
    let mut constructed = Vec::new();
    let mut push_buttons = BTreeSet::new();
    let mut removed = 0_usize;
    for missing in pdf_archive::annotations_without_an_appearance(document, target) {
        let annotation = missing
            .at
            .ok_or(Because::NotBuiltYet(APPEARANCE_ON_A_DIRECT_ANNOTATION))?;
        if forbidden.is_some_and(|forbidden| forbidden.at.contains(&annotation)) {
            removed = removed.saturating_add(1);
            continue;
        }
        let dict = document
            .get(annotation)
            .as_dict()
            .cloned()
            .ok_or(Because::TheFence(APPEARANCE_NOT_DERIVABLE))?;
        // Every button that passes this is a push button, the one kind whose state names no value.
        let button = button_widget(document, &dict);
        if button && !sites::push_button(document, &dict) {
            return Err(Because::NotBuiltYet(BUTTON_APPEARANCE_STATES));
        }
        let built = pdf_model::appearance::for_annotation(document, &dict)
            .ok_or(Because::TheFence(APPEARANCE_NOT_DERIVABLE))?;
        // Two refusals rather than one, because they are two facts and only one of them is about
        // this program: a construction that put marks on the page and owes an entry beside them is
        // partial, and one that put none is a subtype whose clause states no artwork anywhere.
        // The sentence a person reads is the difference, and the second is ADR 0816's fence.
        if built.owed.is_some() {
            return Err(Because::TheFence(if built.drawn {
                APPEARANCE_INCOMPLETE
            } else {
                APPEARANCE_NOT_DERIVABLE
            }));
        }
        let stream = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        at.insert(annotation, stream);
        if button {
            push_buttons.insert(annotation);
        }
        let form = streams_numbered(document, spare, &mut written, built.stream)?;
        written.push((stream, form));
        constructed.push(WrittenAppearance {
            page: missing.page,
            subtype: missing
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
        });
    }
    Ok(Appearances {
        at,
        written,
        push_buttons,
        constructed,
        removed,
    })
}

/// The widget appearances Table 224's flag asks a processor to build, and the fields it could not.
///
/// ISO 19005-2 section 6.4.1 and ISO 19005-4 section 6.4.1 require `/NeedAppearances` to be
/// absent or false, and §12.7.3's Table 224 states the condition under which a writer may say so:
///
/// > A PDF writer shall include this key, with a value of true , if it has not provided appearance
/// > streams for all visible widget annotations present in the document.
///
/// So clearing the flag is a claim about the file, and the claim is true exactly where every
/// visible widget has a normal appearance. [`Self::at`] is what this conversion writes to make it
/// true; [`Self::unconstructed`] is every field where it could not, and a site with anything in it
/// stays refused rather than clearing a flag the file would then be lying with.
#[derive(Debug, Default)]
pub(in crate::archive) struct FieldAppearances {
    /// The `/AP` `/N` object each widget is to name, in the *source's* numbering.
    pub(in crate::archive) at: BTreeMap<ObjectId, ObjectId>,
    /// The form `XObject`s this conversion adds.
    pub(in crate::archive) written: Vec<(ObjectId, Object)>,
    /// Every appearance written, for the report, on the same footing as [`Appearances`]'.
    pub(in crate::archive) constructed: Vec<WrittenAppearance>,
    /// The fields whose widget this conversion could not give an appearance, by name.
    ///
    /// Named by *field* rather than by widget for [`pdf_model::view::Written::unconstructed`]'s
    /// reason: §12.7.4.1 lets one field spread over two widgets, and what a person looks for in a
    /// form is the field's name. The sentence the site is refused with says where to read them.
    pub(in crate::archive) unconstructed: Vec<String>,
}

/// Constructs the appearance of every visible widget the file provides none for.
///
/// **The population is the writer obligation's own, and nothing wider.** Table 224 asks for
/// streams for the *visible* widgets, so §12.5.3's Table 167 decides which those are — a widget
/// whose `Hidden` or `NoView` bit is set is one no reader draws, and giving it marks would be this
/// converter deciding it should be seen. And a widget whose `/AP` already states an `/N` has been
/// provided for: the producer's bytes stay, because §12.7.2 makes the consistency of a stated
/// stream with its field's value the producer's obligation and the file kept it.
///
/// `already` is the population the section 6.3.3 rewrite writes for, passed in so that a widget
/// with no appearance dictionary at all is written once rather than by two rewrites racing for
/// the same key.
///
/// **A button field's widget is never constructed for.** §12.7.5.2.3 and §12.7.5.2.4 make a check
/// box's and a radio button's states "defined by an appearance stream in the appearance dictionary
/// of the field's widget annotation", which its `/V` selects among — so one stream written into
/// `/N` would replace a set of states with a picture, and nothing in a file that states none says
/// what the states look like. Such a widget is named in [`FieldAppearances::unconstructed`] and
/// the site keeps its refusal, which is `doc/adr/1257`.
pub(super) fn prepare_field_appearances(
    document: &Document,
    target: Target,
    already: Option<&Appearances>,
    spare: &mut Spare,
) -> Result<FieldAppearances, Because> {
    let names = pdf_model::view::widgets_by_field_name(document);
    let mut found = FieldAppearances::default();
    for widget in pdf_archive::visible_widgets(document, target) {
        let annotation = widget
            .at
            .ok_or(Because::NotBuiltYet(APPEARANCE_ON_A_DIRECT_ANNOTATION))?;
        if already.is_some_and(|written| written.at.contains_key(&annotation)) {
            continue;
        }
        let name = || field_named(&names, annotation);
        let Some(dict) = document.get(annotation).as_dict().cloned() else {
            found.unconstructed.push(name());
            continue;
        };
        if button_widget(document, &dict) {
            found.unconstructed.push(name());
            continue;
        }
        let built = pdf_model::appearance::for_annotation(document, &dict);
        let Some(built) = built.filter(|built| built.owed.is_none()) else {
            found.unconstructed.push(name());
            continue;
        };
        let stream = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
        found.at.insert(annotation, stream);
        let form = streams_numbered(document, spare, &mut found.written, built.stream)?;
        found.written.push((stream, form));
        found.constructed.push(WrittenAppearance {
            page: widget.page,
            subtype: "Widget".to_owned(),
        });
    }
    found.unconstructed.sort_unstable();
    found.unconstructed.dedup();
    Ok(found)
}

/// A constructed appearance with every stream held inside its dictionary given a number of its own.
///
/// §7.3.8.1: "\[a]ll streams shall be indirect objects". A value set in a face from this machine
/// comes with its font's program, `/CIDToGIDMap` and `/ToUnicode` as streams held directly in the
/// font's dictionaries (ADR 1425); each is written here under a spare number and the reference
/// takes its place. The appearance itself keeps the number its caller took for it.
fn streams_numbered(
    document: &Document,
    spare: &mut Spare,
    written: &mut Vec<(ObjectId, Object)>,
    form: Object,
) -> Result<Object, Because> {
    fn within(
        document: &Document,
        spare: &mut Spare,
        written: &mut Vec<(ObjectId, Object)>,
        value: Object,
    ) -> Result<Object, Because> {
        Ok(match value {
            Object::Stream(stream) => {
                let dict = entries(document, spare, written, &stream.dict)?;
                let id = spare
                    .take(document)
                    .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
                written.push((
                    id,
                    Object::Stream(std::sync::Arc::new(Stream {
                        dict,
                        data: std::sync::Arc::clone(&stream.data),
                        decryption_failed: false,
                    })),
                ));
                Object::Reference(id)
            }
            Object::Dictionary(dict) => {
                Object::Dictionary(entries(document, spare, written, &dict)?)
            }
            Object::Array(items) => Object::Array(
                items
                    .into_iter()
                    .map(|item| within(document, spare, written, item))
                    .collect::<Result<_, _>>()?,
            ),
            other => other,
        })
    }
    fn entries(
        document: &Document,
        spare: &mut Spare,
        written: &mut Vec<(ObjectId, Object)>,
        dict: &Dictionary,
    ) -> Result<Dictionary, Because> {
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            out.insert(
                key.clone(),
                within(document, spare, written, value.clone())?,
            );
        }
        Ok(out)
    }
    match form {
        Object::Stream(stream) => Ok(Object::Stream(std::sync::Arc::new(Stream {
            dict: entries(document, spare, written, &stream.dict)?,
            data: std::sync::Arc::clone(&stream.data),
            decryption_failed: stream.decryption_failed,
        }))),
        other => Ok(other),
    }
}

/// §12.7.4.2's fully qualified name of the field a widget belongs to.
///
/// The table is [`pdf_model::view::widgets_by_field_name`]'s, which is the reading the viewer
/// names a field by, so a person reading the report sees the name their form shows them. A widget
/// no field tree reaches is named by its object number, which is what there is to say about it.
fn field_named(names: &BTreeMap<String, Vec<ObjectId>>, widget: ObjectId) -> String {
    names
        .iter()
        .find(|(_, widgets)| widgets.contains(&widget))
        .map_or_else(
            || format!("the field of object {}", widget.number),
            |(name, _)| name.clone(),
        )
}

/// Why an annotation nothing can name is not removed.
///
/// §7.7.3.3's Table 31 requires `/Annots` to "contain indirect references to all annotations
/// associated with the page", and files exist that write a dictionary straight into the array
/// instead. This rewrite acts on objects, so there is nothing for it to take out;
/// `prepare_appearances` refuses the same case for the same reason.
const REMOVAL_OF_A_DIRECT_ANNOTATION: &str = "an annotation of a subtype ISO 19005 does not admit \
     is written directly into its page's Annots array rather than being an object of its own, and \
     the rewrite that removes one acts on objects. Nothing here can reach it";

/// The annotations the two section 6.3.1 rows are about, read off the document once.
pub(super) fn prepare_forbidden_annotations(
    document: &Document,
    target: Target,
) -> Result<ForbiddenAnnotations, Because> {
    let mut at = BTreeSet::new();
    let mut removed = Vec::new();
    // ISO 19005-4 section 6.3.1's 3D and RichMedia annotations outside PDF/A-4e are the same
    // removal under the same authorisation (`doc/adr/2051` section 4): one population, so that a
    // page losing both kinds is rewritten once.
    // `FileAttachment` outside PDF/A-4f is the same removal again, its file kept in the name tree
    // by [`the_attached_files`] (`doc/adr/2067` section 4).
    let outside_their_flavour =
        pdf_archive::annotations_outside_the_engineering_flavour(document, target)
            .into_iter()
            .chain(pdf_archive::file_attachments_outside_their_flavour(
                document, target,
            ));
    for forbidden in pdf_archive::annotations_of_a_forbidden_subtype(document, target)
        .into_iter()
        .chain(outside_their_flavour)
    {
        let annotation = forbidden
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_OF_A_DIRECT_ANNOTATION))?;
        at.insert(annotation);
        // §12.5.6.2's Table 172 gives a markup annotation a `/Popup` naming the window it opens,
        // and that window is an annotation of the page in its own right. It goes with its parent
        // rather than being left pointing at an object the output does not hold.
        if let Some(dict) = document.get(annotation).as_dict()
            && let Some(popup) = dict.get("Popup").and_then(Object::as_reference)
        {
            at.insert(popup);
        }
        removed.push(RemovedAnnotation {
            at: annotation,
            page: forbidden.page,
            subtype: forbidden
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
            appearance: forbidden.normal_appearance,
            flags: None,
        });
    }
    Ok(ForbiddenAnnotations { at, removed })
}

/// The annotations whose stated `/F` ISO 19005 forbids, and what removing each costs.
///
/// [`ForbiddenAnnotations`]' shape over ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2's
/// population rather than section 6.3.1's, and a type of its own because the two removals are two
/// authorisations: an operator who accepts losing a `Movie` annotation has said nothing about the
/// review comment its producer hid.
#[derive(Debug)]
pub(in crate::archive) struct HiddenAnnotations {
    /// The kept part of an explicitly shaped plan; absent for an unchanged site-wide policy.
    pub(in crate::archive) kept: Option<Vec<RemovedAnnotation>>,
    /// The objects to take out of their pages' `/Annots` arrays.
    ///
    /// The popup of a removed annotation is in here beside it, on
    /// [`ForbiddenAnnotations::at`]'s reading of §12.5.6.14.
    pub(in crate::archive) at: BTreeSet<ObjectId>,
    /// Every annotation removed, in page order, for the report.
    pub(in crate::archive) removed: Vec<RemovedAnnotation>,
}

/// Why an annotation nothing can name is not removed for its flags.
///
/// [`REMOVAL_OF_A_DIRECT_ANNOTATION`]'s case at the other removal's site: §7.7.3.3's Table 31
/// requires `/Annots` to hold indirect references, files exist that write a dictionary straight
/// into the array, and a rewrite that acts on objects has nothing to take out.
const REMOVAL_OF_A_DIRECT_HIDDEN_ANNOTATION: &str = "an annotation whose F entry ISO 19005 \
     forbids is written directly into its page's Annots array rather than being an object of its \
     own, and the rewrites that remove one or show it act on objects. Nothing here can reach it";

/// The annotations the two section 6.3.2 flag rows are about, read off the document once.
pub(super) fn prepare_hidden_annotations(
    document: &Document,
    target: Target,
    answers: Option<super::super::config::AnnotationFlagAnswers>,
) -> Result<HiddenAnnotations, Because> {
    if answers.is_some() {
        super::super::in_place::validate_flag_types(document)?;
    }
    let mut at = BTreeSet::new();
    let mut removed = Vec::new();
    for hidden in pdf_archive::annotations_the_flags_forbid(document, target) {
        let annotation = hidden
            .at
            .ok_or(Because::NotBuiltYet(REMOVAL_OF_A_DIRECT_HIDDEN_ANNOTATION))?;
        at.insert(annotation);
        if let Some(dict) = document.get(annotation).as_dict()
            && let Some(popup) = dict.get("Popup").and_then(Object::as_reference)
        {
            at.insert(popup);
        }
        removed.push(RemovedAnnotation {
            at: annotation,
            page: hidden.page,
            subtype: hidden
                .subtype
                .unwrap_or_else(|| "an annotation stating no subtype".to_owned()),
            appearance: hidden.normal_appearance,
            flags: Some(hidden.flags),
        });
    }
    if let Some(answers) = answers {
        return super::super::in_place::partition_flags(document, answers, removed);
    }
    Ok(HiddenAnnotations {
        at,
        removed,
        kept: None,
    })
}

/// The appearance dictionaries that lose every key but `/N`.
#[derive(Debug, Default)]
pub(in crate::archive) struct ExtraAppearanceStates {
    /// The annotation objects whose `/AP` is rewritten to hold `/N` alone.
    pub(in crate::archive) at: BTreeSet<ObjectId>,
}

/// Why an appearance dictionary with nothing to fall back to is not reduced.
///
/// §12.5.5's Table 170 gives `/R` and `/D` the default "the value of the N entry", which is what
/// makes dropping them a loss of artwork rather than of behaviour. Where the dictionary states no
/// `/N`, that default names nothing, and the reduction would leave an annotation carrying an
/// appearance dictionary that describes no appearance.
const NO_NORMAL_APPEARANCE_TO_FALL_BACK_TO: &str = "this annotation's appearance dictionary \
     states a rollover or down appearance and no normal one. ISO 32000-2 Table 170 makes the \
     normal appearance the default of both keys ISO 19005 section 6.3.3 forbids, so there is \
     nothing here for a reader to fall back to and dropping them would leave an appearance \
     dictionary describing no appearance";

/// Why an appearance dictionary nothing can name is not reduced.
const REDUCTION_OF_A_DIRECT_ANNOTATION: &str = "an annotation whose appearance dictionary states \
     more than the normal appearance is written directly into its page's Annots array rather \
     than being an object of its own, and this rewrite acts on objects. Nothing here can reach it";

/// The annotations the two section 6.3.3 appearance-dictionary rows are about.
pub(super) fn prepare_extra_appearance_states(
    document: &Document,
    target: Target,
) -> Result<ExtraAppearanceStates, Because> {
    let mut at = BTreeSet::new();
    for extra in pdf_archive::annotations_with_extra_appearance_states(document, target) {
        if !extra.normal {
            return Err(Because::NotBuiltYet(NO_NORMAL_APPEARANCE_TO_FALL_BACK_TO));
        }
        at.insert(
            extra
                .at
                .ok_or(Because::NotBuiltYet(REDUCTION_OF_A_DIRECT_ANNOTATION))?,
        );
    }
    Ok(ExtraAppearanceStates { at })
}

/// Whether this annotation is the widget of a button field.
///
/// §12.7.4.1's Table 228 makes `/FT` inheritable, and `TechNote 0010`'s A023 resolves that an
/// unmerged widget's field type is read from the form field dictionary above it — which is why
/// this walks `/Parent` rather than reading the key off the annotation alone.
fn button_widget(document: &Document, annotation: &Dictionary) -> bool {
    if document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Widget")
    {
        return false;
    }
    let mut node = annotation.clone();
    for _ in 0..MAX_FIELD_DEPTH {
        if let Some(kind) = document.get_key(&node, "FT").as_name() {
            return kind.as_bytes() == b"Btn";
        }
        let Some(parent) = node.get("Parent").and_then(Object::as_reference) else {
            return false;
        };
        let Some(above) = document.get(parent).as_dict().cloned() else {
            return false;
        };
        node = above;
    }
    false
}

/// What became of each embedded file the embedded-file requirements failed at.
///
/// The preservation's own list where a `preserve` by page answered the site; otherwise one
/// removal per file where the plan authorises `embedded-file` and neither derives nor preserves
/// the site, which is what the table's `discard` does to each. A site answered by derivation
/// reports through `Derived` rows instead, and one answered `stop` writes no output.
pub(super) fn attachment_outcomes(
    plan: &ArchivePlan,
    failed: &BTreeSet<&'static str>,
    embedded: Option<&sites::EmbeddedFiles>,
    preserved: Option<&Composed>,
) -> Vec<super::super::report::AttachmentOutcome> {
    if let Some(composed) = preserved
        && !composed.attachments.is_empty()
    {
        return composed.attachments.clone();
    }
    let Some(files) = embedded else {
        return Vec::new();
    };
    let Some(site) = ATTACHMENT_REQUIREMENTS.into_iter().find(|site| {
        failed.contains(site)
            && !plan.derivations.iter().any(|row| row.site == *site)
            && !plan.preservations.iter().any(|row| row.site == *site)
    }) else {
        return Vec::new();
    };
    if !plan.authorised.grants(Loss::EmbeddedFile) {
        return Vec::new();
    }
    files
        .each
        .iter()
        .map(|(at, name)| super::super::report::AttachmentOutcome {
            site,
            object: *at,
            name: name.clone(),
            annotations: Vec::new(),
            answer: super::super::report::AttachmentAnswer::Removed(
                super::super::report::DISCARDED_BY_THE_ROW.to_owned(),
            ),
            stated_on: None,
        })
        .collect()
}

/// Whether `derive:<tool>:append` converted a file of `site` whose pages are to be appended.
pub(super) fn appends_derived_pages(
    remedies: &super::super::remedies::Remedies,
    site: &str,
) -> bool {
    remedies.appended.values().any(|file| file.site == site)
}

/// The files a `preserve` remedy attaches, or the reason it cannot.
///
/// `doc/adr/1270`. Two sites reach here and the bytes are the producer's at both: the XMP packet
/// [`Rewrite::FreshMetadataPacket`](super::super::rewrite::Rewrite::FreshMetadataPacket) replaces, kept under ISO 19005-2 section 6.6.2.1's row with
/// `original = "attach"`, and the XFA resource [`Rewrite::XfaRemoved`](super::super::rewrite::Rewrite::XfaRemoved) takes out of the
/// interactive form dictionary, kept under section 6.4.2's row with `keep-xfa = "attach"`.
///
/// **Nothing is attached that no failed requirement asked for**, which is `doc/adr/0947`'s first
/// rule: a site the operator answered but this document passed keeps nothing, because there is
/// nothing being taken away.
#[expect(
    clippy::too_many_arguments,
    reason = "each is one source of what the tree holds — the packets, the XFA resource, the \
              files a removed annotation named — and the catalog and the spare numbers they are \
              filed with; a struct bundling them would be this function's arguments renamed"
)]
pub(super) fn the_attached_files(
    plan: &ArchivePlan,
    document: &Document,
    catalog: Option<&Dictionary>,
    failed: &BTreeSet<&'static str>,
    fresh: Result<&Fresh, &Because>,
    xfa: Option<Site>,
    forbidden: Option<&ForbiddenAnnotations>,
    embedded: Option<&sites::EmbeddedFiles>,
    spare: &mut Spare,
) -> Result<super::super::attach::Attached, Because> {
    let asked = |site: &'static str| {
        plan.preservations
            .iter()
            .any(|preservation| preservation.site == site && preservation.by_attachment)
            && failed.contains(site)
    };
    // One attachment per replaced stream, under the first of the three rows the operator answered
    // and this document failed: one packet's bytes cannot be split between rows that all named
    // the same stream. The same rule `the_preserved_pages` reads for the page.
    let packet_site = PACKET_REQUIREMENTS.into_iter().find(|site| asked(site));
    let packets: Vec<(ObjectId, std::sync::Arc<[u8]>)> = match packet_site {
        Some(_) => {
            // The replacement is what makes the file conform and the attachment is what keeps
            // what it replaces, so a document whose packets cannot be replaced gets neither.
            let fresh = fresh.map_err(|because| *because)?;
            let mut out = Vec::new();
            for at in fresh.packets.keys() {
                let Object::Stream(stream) = document.get(*at) else {
                    return Err(Because::NotBuiltYet(NOT_A_PACKET));
                };
                // A stream whose data will not decode has no bytes to keep, and the replacement
                // still happens — the same give-up the appended page makes, for its reason.
                let Some(bytes) = document.decoded_stream_data(&stream) else {
                    continue;
                };
                out.push((*at, bytes));
            }
            out
        }
        None => Vec::new(),
    };
    let resource = if asked(XFA_REQUIREMENT) {
        // The removal is what makes the file conform and the attachment is what keeps what it
        // removes, so a document whose `/XFA` cannot be removed gets neither.
        let site = xfa.ok_or(Because::NotBuiltYet(XFA_NOT_REMOVABLE))?;
        let form = match site {
            Site::InCatalog => {
                catalog.and_then(|catalog| document.get_key(catalog, "AcroForm").as_dict().cloned())
            }
            Site::Object(at) => document.get(at).as_dict().cloned(),
        };
        let form = form.ok_or(Because::NotBuiltYet(XFA_NOT_REMOVABLE))?;
        Some(
            super::super::attach::xfa_resource(document, &form)
                .ok_or(Because::NotBuiltYet(XFA_NOT_READABLE))?,
        )
    } else {
        None
    };
    let mut filings =
        kept_attachments(document, failed, forbidden, embedded.map(|files| &files.at));
    filings.extend(kept_media(document, plan, failed, forbidden));
    if packets.is_empty() && resource.is_none() && filings.is_empty() {
        return Err(Because::NotBuiltYet(NOT_ASKED_FOR));
    }
    let mut keeps: Vec<super::super::attach::Keeping<'_>> = Vec::new();
    if let Some(site) = packet_site {
        keeps.extend(
            packets
                .iter()
                .map(|(at, bytes)| super::super::attach::Keeping {
                    site,
                    name: super::super::attach::filed_as("original-metadata", *at, "xmp"),
                    description: format!(
                        "the XMP metadata packet object {} {} held, as the producer wrote it",
                        at.number, at.generation
                    ),
                    bytes,
                }),
        );
    }
    if let Some(bytes) = resource.as_deref() {
        keeps.push(super::super::attach::Keeping {
            site: XFA_REQUIREMENT,
            name: XFA_FILE_NAME.to_owned(),
            description: "the XFA resource the interactive form dictionary held, as the producer \
                          wrote it"
                .to_owned(),
            bytes,
        });
    }
    super::super::attach::attach(document, catalog, &keeps, &filings, spare)
}

/// ISO 19005-4 section 6.3.1's `FileAttachment` row: the files its removed annotations named.
///
/// Only where the row failed, so a `FileAttachment` removed for another reason — a flag section
/// 6.3.2 forbids — is not the population. The file's name is §7.11.3's `/UF` where the
/// specification states one and its `/F` otherwise, as `pdf_model::attachment` reads it; one that
/// states neither is filed as `attachment` (`doc/adr/2067` section 4).
///
/// **A file an embedded-file row answers is that row's, not this one's** (`doc/adr/2536`): where
/// ISO 19005-2 section 6.8 or ISO 19005-4 section 6.9 failed at the specification the annotation
/// names, the row removes it, lays it out as pages or derives it, and filing it again here would
/// put back in the name tree the file that row took out.
fn kept_attachments(
    document: &Document,
    failed: &BTreeSet<&'static str>,
    forbidden: Option<&ForbiddenAnnotations>,
    answered_by_their_own_row: Option<&BTreeSet<ObjectId>>,
) -> Vec<super::super::attach::Filing> {
    const SITE: &str = "annotations/file-attachment-only-in-embedded-file-files";
    let Some(forbidden) = forbidden.filter(|_| failed.contains(SITE)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for removed in &forbidden.removed {
        if removed.subtype != "FileAttachment" {
            continue;
        }
        let Some(annotation) = document.get(removed.at).as_dict().cloned() else {
            continue;
        };
        let Some(specification) = annotation.get("FS").cloned() else {
            continue;
        };
        if specification.as_reference().is_some_and(|at| {
            answered_by_their_own_row.is_some_and(|answered| answered.contains(&at))
        }) {
            continue;
        }
        let resolved = document.resolve(&specification);
        let name = match &resolved {
            Object::Dictionary(dict) => ["UF", "F"].iter().find_map(|key| {
                document
                    .get_key(dict, key)
                    .as_string()
                    .map(pdf_syntax::text_string)
                    .filter(|name| !name.is_empty())
            }),
            Object::String(bytes) => Some(pdf_syntax::text_string(bytes)),
            _ => None,
        }
        .unwrap_or_else(|| "attachment".to_owned());
        out.push(super::super::attach::Filing {
            site: SITE,
            annotation: removed.at,
            subtype: "FileAttachment",
            name,
            specification,
        });
    }
    out
}

/// ISO 19005-4 section 6.3.1's Screen and Movie rows at PDF/A-4f and PDF/A-4e: the clip each
/// removed annotation plays, kept as the embedded file it already is (`keep-media = "attach"`).
///
/// The clip is reached by the dictionaries ISO 32000-2 states and no further: a Screen
/// annotation's `/A` (§12.5.6.18) where it is a §12.6.4.14 rendition action, its `/R` a §13.2.3.2
/// media rendition, that rendition's `/C` a media clip — a §13.2.4.3 section's `/D` followed once
/// to the §13.2.4.2 data it narrows — and the data's `/D`; a Movie annotation's `/Movie`
/// dictionary's `/F` (§13.4). Only a specification holding an `/EF` is filed: §13.2.4.2's `/D` may
/// name a file outside this one, and the `/EmbeddedFiles` tree files only what the document holds.
/// A clip the walk does not reach is not filed, and the removal still happens — the poster is
/// what the row's `preserve` keeps, and the report's kept files say which clips came with it.
fn kept_media(
    document: &Document,
    plan: &ArchivePlan,
    failed: &BTreeSet<&'static str>,
    forbidden: Option<&ForbiddenAnnotations>,
) -> Vec<super::super::attach::Filing> {
    let Some(site) = super::super::config::MEDIA_SITES.into_iter().find(|site| {
        failed.contains(site)
            && plan
                .preservations
                .iter()
                .any(|preservation| preservation.site == *site && preservation.keeps_media)
    }) else {
        return Vec::new();
    };
    let Some(forbidden) = forbidden else {
        return Vec::new();
    };
    let dictionary = |object: &Object| document.resolve(object).as_dict().cloned();
    let is = |dict: &Dictionary, word: &[u8]| {
        document
            .get_key(dict, "S")
            .as_name()
            .is_some_and(|name| name.as_bytes() == word)
    };
    let mut out = Vec::new();
    for removed in &forbidden.removed {
        let Some(annotation) = document.get(removed.at).as_dict().cloned() else {
            continue;
        };
        let (subtype, specification) = match removed.subtype.as_str() {
            "Screen" => {
                let clip = annotation
                    .get("A")
                    .and_then(dictionary)
                    .filter(|action| is(action, b"Rendition"))
                    .and_then(|action| action.get("R").and_then(dictionary))
                    .filter(|rendition| is(rendition, b"MR"))
                    .and_then(|rendition| rendition.get("C").and_then(dictionary));
                let data = clip.and_then(|clip| {
                    if is(&clip, b"MCS") {
                        clip.get("D").and_then(dictionary)
                    } else {
                        Some(clip)
                    }
                });
                (
                    "Screen",
                    data.filter(|data| is(data, b"MCD"))
                        .and_then(|data| data.get("D").cloned()),
                )
            }
            "Movie" => (
                "Movie",
                annotation
                    .get("Movie")
                    .and_then(dictionary)
                    .and_then(|movie| movie.get("F").cloned()),
            ),
            _ => continue,
        };
        let Some(specification) = specification else {
            continue;
        };
        let Some(held) = dictionary(&specification).filter(|spec| spec.get("EF").is_some()) else {
            continue;
        };
        let name = ["UF", "F"]
            .iter()
            .find_map(|key| {
                document
                    .get_key(&held, key)
                    .as_string()
                    .map(pdf_syntax::text_string)
                    .filter(|name| !name.is_empty())
            })
            .unwrap_or_else(|| "media clip".to_owned());
        out.push(super::super::attach::Filing {
            site,
            annotation: removed.at,
            subtype,
            name,
            specification,
        });
    }
    out
}

/// ISO 19005-2 section 6.4.2's row, whose attachment is the XFA resource.
const XFA_REQUIREMENT: &str = "forms/no-xfa-key";

/// The name the XFA resource is filed under.
///
/// Not [`super::super::attach::filed_as`]'s shape, because there is exactly one interactive form
/// dictionary in a document (§7.7.2's Table 28 makes `/AcroForm` one entry) and therefore exactly
/// one resource: a number would name an object the tree does not otherwise mention. The extension
/// is the one §12.7.3's Table 224 names the format by — "whose format shall conform to the Data
/// Package (XDP) Specification" — and is a convention rather than a declaration, which is why the
/// media type says what it says.
const XFA_FILE_NAME: &str = "original-xfa.xdp";

/// Why an XFA resource this conversion cannot reach stops the attachment.
const XFA_NOT_REMOVABLE: &str = "this configuration keeps the XFA resource as an embedded file, \
     and the interactive form dictionary the catalog's AcroForm names is not one this conversion \
     can reach — so there is no resource to keep and the removal beside it does not happen either";

/// Why an XFA resource this conversion cannot read stops the attachment.
const XFA_NOT_READABLE: &str = "this configuration keeps the XFA resource as an embedded file, \
     and §12.7.3's Table 224 makes the entry a stream or an array of packets whose streams are \
     the resource end to end. This document's XFA is neither shape, or one of its streams will \
     not decode — so what could be attached is part of a resource rather than the resource, and \
     the removal beside it does not happen either";

#[cfg(test)]
mod tests {
    use super::{Spare, streams_numbered};
    use pdf_syntax::Document;
    use pdf_syntax::object::{Dictionary, Name, Object, Stream};
    use std::sync::Arc;

    /// A one-object document, whose only number is 1.
    fn document() -> Document {
        let object = "1 0 obj\n<< /Type /Catalog >>\nendobj\n";
        let head = "%PDF-1.7\n";
        let xref = head.len().saturating_add(object.len());
        let text = format!(
            "{head}{object}xref\n0 2\n0000000000 65535 f \n{:010} 00000 n \n\
             trailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            head.len()
        );
        Document::open(text.into_bytes()).expect("the fixture is a valid PDF")
    }

    fn stream(dict: Dictionary, data: &[u8]) -> Object {
        Object::Stream(Arc::new(Stream {
            dict,
            data: data.into(),
            decryption_failed: false,
        }))
    }

    /// §7.3.8.1: a program held directly inside a constructed appearance's font is written as an
    /// object of its own and named by reference, and the appearance itself keeps its body (ADR 1425).
    #[test]
    fn a_stream_inside_a_constructed_appearance_is_given_a_number() {
        let document = document();
        let mut descriptor = Dictionary::new();
        descriptor.insert(
            Name::new(&b"FontFile2"[..]),
            stream(Dictionary::new(), b"program"),
        );
        let mut resources = Dictionary::new();
        resources.insert(Name::new(&b"Font"[..]), Object::Dictionary(descriptor));
        let mut form = Dictionary::new();
        form.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
        let mut spare = Spare::of(&document);
        let mut written = Vec::new();
        let numbered = streams_numbered(&document, &mut spare, &mut written, stream(form, b"q Q"))
            .expect("spare numbers");

        assert_eq!(written.len(), 1, "the program, and only it");
        let (id, program) = &written[0];
        assert_eq!(id.number, 2, "past the document's own numbers");
        assert_eq!(program.as_stream().map(|s| &*s.data), Some(&b"program"[..]));
        let Object::Stream(form) = numbered else {
            panic!("the appearance stays a stream");
        };
        assert_eq!(&*form.data, b"q Q");
        let reference = form
            .dict
            .get("Resources")
            .and_then(Object::as_dict)
            .and_then(|resources| resources.get("Font"))
            .and_then(Object::as_dict)
            .and_then(|descriptor| descriptor.get("FontFile2"))
            .and_then(Object::as_reference);
        assert_eq!(reference, Some(*id));
    }
}
