//! A `derive` remedy's second stage: the PDF a declared program returned, converted by this
//! converter before anything of it reaches the archive.
//!
//! `doc/rfc/0014` section 5 makes `derive` three stages, and this file is the middle one. The
//! first is the operator's program (`crate::tool`, `doc/questions/A54`): it turns an attachment
//! this tree cannot read into a PDF. **What it returns is an untrusted input like any other** —
//! the program read a file the document carried, and nothing promises that what it wrote conforms
//! to anything — so it is opened here by the same reader, under the same limits, and converted by
//! the same converter with the same configuration, to the part the outer target admits:
//!
//! - **kept an attachment** (`placement = "attach"`, the default), to a part ISO 19005-2
//!   section 6.8 or ISO 19005-4 section 6.9 admits an embedded file conforming to. Section 6.8
//!   admits part 1 or part 2 at any level, so a part-2 target's attachment is converted to
//!   PDF/A-2b, the level whose requirements every other level of part 2 includes; section 6.9
//!   admits parts 1, 2 and 4, so a part-4 target's attachment is converted to plain PDF/A-4;
//! - **laid out as pages** (`placement = "append"`), to the outer target itself, because the
//!   pages become the archive's own and are held to everything its other pages are.
//!
//! # One level, and what bounds it
//!
//! The inner conversion is the outer one's plan with the target swapped and every remedy that
//! names a program removed: an attachment *inside* the derived PDF is the inner run's to answer
//! with what the configuration answers without a program, and **no second program runs**. So the
//! recursion is one level by the plan's shape rather than by a counter. The inner run converts
//! once — no `settle` pass inside it — and its refusal is the outer site's, by name.
//!
//! **The budget is divided, not doubled.** Every derived file one conversion holds — each inner
//! output, kept until the archive is written — is charged against one stream's ceiling of the
//! caller's [`Budget`], `Limits::max_stream_len`, which is the ceiling the archive's own reader
//! holds each of those streams to. The inner document is opened under what is left of it, and a
//! file past what is left is refused by name before it is opened (`doc/adr/2539`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use pdf_archive::survey::DeviceFamily;
use pdf_archive::{Flavour, Level, Target};
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use pdf_syntax::{Document, Limits};

use super::COMPRESSION_LEVEL;
use super::config::Placement;
use super::decision::Because;
use super::prepare::Spare;
use super::preserve::Marks;
use super::{ArchivePlan, Conversion, Report, run_pass};
use crate::tool::ToolOutputs;
use crate::{Budget, MemorySinks, Sinks};

/// What one derived PDF became under this converter.
pub(super) struct Converted {
    /// The inner conversion's file.
    pub(super) bytes: Vec<u8>,
    /// The part it was converted to.
    pub(super) target: Target,
    /// The inner conversion's own report, for what it decided.
    pub(super) conversion: Conversion,
}

/// The part a derived file is converted to, for the outer target and where it is placed.
///
/// See the module's comment: the least demanding part the outer target's clause admits for an
/// attachment, and the outer target itself for pages.
pub(super) const fn target_for(outer: Target, placement: Placement) -> Target {
    match placement {
        Placement::Append => outer,
        Placement::Attach => match outer {
            Target::Two(_) => Target::Two(Level::B),
            Target::Four(_) => Target::Four(Flavour::Plain),
        },
    }
}

/// Converts the PDF a declared program returned, or says in a sentence why it was not.
///
/// `held` is what earlier derived files of this conversion already hold of the budget, and grows
/// by this file's output where it converts. `what` names the file in the sentences: the PDF a
/// tool returned, or an embedded file that is itself a PDF and is drawn as pages
/// (`super::drawn`), which is converted the same way and charged against the same budget.
///
/// # Errors
///
/// A sentence naming the reason: the file past what is left of the budget, a file that does not
/// open, a refusal of the inner conversion with every requirement that stopped it, or an output
/// past the budget.
pub(super) fn convert(
    plan: &ArchivePlan,
    budget: &Budget,
    held: &mut usize,
    bytes: &[u8],
    placement: Placement,
    what: &str,
) -> Result<Converted, String> {
    let target = target_for(plan.target, placement);
    let remaining = budget.limits.max_stream_len.saturating_sub(*held);
    if bytes.len() > remaining {
        return Err(past_the_budget(bytes.len(), remaining, *held));
    }
    let mut child = *budget;
    child.limits.max_stream_len = remaining;
    let document = Document::open_with_limits(Arc::<[u8]>::from(bytes), child.limits)
        .map_err(|error| format!("{what} does not open: {error}"))?;
    let inner = ArchivePlan {
        source: 0,
        target,
        derivations: Vec::new(),
        resolutions: Vec::new(),
        tool_outputs: ToolOutputs::new(),
        external_data: BTreeMap::new(),
        // One level: a PDF embedded in this one is the inner run's to answer, and drawing it
        // would be a conversion inside a conversion with no bound but the budget's.
        preservations: plan
            .preservations
            .iter()
            .cloned()
            .map(|preservation| super::Preservation {
                draws_documents: false,
                ..preservation
            })
            .collect(),
        ..plan.clone()
    };
    let sinks = MemorySinks::new();
    let mut report = Report::default();
    run_pass(
        &inner,
        0,
        std::slice::from_ref(&document),
        &sinks as &dyn Sinks,
        &mut report,
        false,
        &child,
    )
    .map_err(|refusal| format!("{what} does not convert to {target}: {refusal}"))?;
    if !report.requested.is_empty() {
        // The inner plan names no program, so nothing can be asked for; a request here would be a
        // second program, which this stage never runs.
        return Err(format!(
            "{what} asks for {} more program run(s), and a derived file is converted with no \
             program of its own (doc/rfc/0014 section 5)",
            report.requested.len()
        ));
    }
    if !report.refused.is_empty() {
        let stopped: Vec<&str> = report
            .refused
            .iter()
            .map(|declined| declined.subject.as_str())
            .collect();
        return Err(format!(
            "{what} does not convert to {target}; what stopped it: {}",
            stopped.join("; ")
        ));
    }
    let conversion = report
        .archive
        .ok_or_else(|| format!("{what} was not converted to {target}"))?;
    let (_, output) = sinks
        .into_outputs()
        .pop()
        .ok_or_else(|| format!("the conversion of {what} to {target} wrote nothing"))?;
    if output.len() > remaining {
        return Err(past_the_budget(output.len(), remaining, *held));
    }
    *held = held.saturating_add(output.len());
    Ok(Converted {
        bytes: output,
        target,
        conversion,
    })
}

/// The sentence a derived file past the budget is refused with.
fn past_the_budget(bytes: usize, remaining: usize, held: usize) -> String {
    format!(
        "the derived file is {bytes} byte(s) and {remaining} byte(s) of this conversion's budget \
         are left for derived files ({held} byte(s) already held by earlier ones); every derived \
         file one conversion holds is charged against one stream's ceiling of the budget \
         (doc/adr/2539)"
    )
}

/// The pages of a derived PDF, written into the source's numbering as form `XObject`s.
///
/// `derive:<tool>:append`'s third stage (`doc/rfc/0014` section 5, `doc/adr/2539`): the file the
/// inner conversion wrote is the program's rendering of the document's own attachment, held to
/// the outer target, and its pages are appended under `doc/adr/1014`'s permission as
/// [`super::preserve::Kept::Marks`] — each one form `XObject` invoked under the identity, on a page
/// stating the derived page's own boxes, so that not a mark of it moves.
pub(super) struct ImportedPages {
    /// Every object the pages reach, in the source's numbering.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// One set of marks per derived page, in page order.
    pub(super) pages: Vec<Marks>,
}

/// How many objects one derived file may bring into the source's numbering.
///
/// `CLAUDE.md` principle 3's explicit bound, checked before each object is taken: the derived
/// file is a program's output, and its object count is the program's to choose. Far past the
/// objects a page of any producer reaches — its fonts, images and patterns — and the file is
/// already within the budget [`convert`] charged it against.
pub(super) const MOST_IMPORTED_OBJECTS: usize = 65_536;

/// Why a derived file's pages are not appended to a document that describes its content.
///
/// The derived pages' structure is the program's, and §14.8.2.2.1 would make marks no
/// marked-content sequence brackets artifacts of a tagged archive: the refusal
/// [`super::tagged::MARKS_IN_A_TAGGED_DOCUMENT`] gives an annotation's marks, for the same reason.
pub(super) const DERIVED_PAGES_IN_A_TAGGED_DOCUMENT: &str = "the derived PDF's pages would be \
     appended to a document that describes its content in a structure tree, and naming a \
     structure type for marks a program drew is what this conversion will not do (ISO 32000-2 \
     Table 355, ISO 19005-2 clause 6.7.1); placement = \"attach\" keeps the derived file instead";

/// Why a derived file's pages are not laid out because the file does not open.
const DERIVED_PAGES_DO_NOT_OPEN: &str = "the PDF this converter converted from the tool's output \
     does not open again, so its pages cannot be appended";

/// Why a derived file's pages are not laid out because there are too many of them.
const DERIVED_PAGES_TOO_MANY: &str = "the derived PDF has more pages than one conversion appends \
     (preserve::MOST_PAGES), so its pages are not appended";

/// Why a derived file's pages are not laid out because a page is not read.
const DERIVED_PAGE_UNREAD: &str = "a page of the derived PDF does not read as a page, so its \
     pages are not appended";

/// Why a derived file's pages are not laid out because a resource names a page.
///
/// A page's resources never reach a page object in a file this converter wrote; one that does
/// would carry the derived document's page tree into the archive's, where nothing places it.
const DERIVED_RESOURCE_NAMES_A_PAGE: &str = "a resource of the derived PDF's pages refers to a \
     page object, which an appended page cannot carry";

/// Why a derived file's pages are not laid out because they reach too many objects.
const DERIVED_PAGES_TOO_LARGE: &str = "the derived PDF's pages reach more objects than one \
     derived file may bring into the archive (MOST_IMPORTED_OBJECTS)";

/// Why a derived file's pages are not laid out because no object number is free.
const DERIVED_NO_NUMBER: &str = "no free object number was found for a derived page's objects";

/// The resource name a derived page's own content is invoked under, inside its wrapper.
const DERIVED_PAGE: &[u8] = b"DerivedPage";

/// The bit §12.5.3's Table 167 gives `Hidden`: "the annotation shall not be displayed or
/// printed".
const HIDDEN: i64 = 1 << 1;

/// The bit §12.5.3's Table 167 gives `NoView`: the annotation shall not be displayed on the
/// screen.
const NO_VIEW: i64 = 1 << 5;

/// Writes every page of `bytes` into the source's numbering, or says why it does not.
///
/// **Each page becomes two form `XObject`s.** The inner one is the derived page's content and
/// resources as they are, so its graphics state is its own whatever the content leaves open
/// (§8.10.1's implicit `q` and `Q` around a form); the outer one invokes it and then, in page
/// order, the normal appearance of each annotation §12.5.3's flags show on a screen, under
/// §12.5.5's matrix — the marks the derived page showed, its annotations' appearances included.
/// What does not come across is the annotations as annotations: a link or a widget of the derived
/// file is the program's, and the archive's pages state none of their own.
///
/// # Errors
///
/// A [`Because`] naming the reason: the file does not open, it has more pages than
/// `most_pages`, a page does not read, a resource refers to a page, the pages reach more than
/// [`MOST_IMPORTED_OBJECTS`], or no object number is free.
pub(super) fn import_pages(
    source: &Document,
    bytes: &[u8],
    limits: Limits,
    most_pages: usize,
    spare: &mut Spare,
) -> Result<ImportedPages, Because> {
    let derived = Document::open_with_limits(Arc::<[u8]>::from(bytes), limits)
        .map_err(|_| Because::NotBuiltYet(DERIVED_PAGES_DO_NOT_OPEN))?;
    let tree = Pages::new(&derived);
    // Before any page is read: the count is the derived file's own to state.
    if tree.len() > most_pages {
        return Err(Because::NotBuiltYet(DERIVED_PAGES_TOO_MANY));
    }
    let mut importer = Importer {
        derived: &derived,
        source,
        spare,
        numbers: BTreeMap::new(),
        queue: Vec::new(),
        written: Vec::new(),
    };
    let mut pages = Vec::new();
    for page in tree.every() {
        let page = page.ok_or(Because::NotBuiltYet(DERIVED_PAGE_UNREAD))?;
        let content = importer.page_form(&page)?;
        let wrapper = importer.wrapper(&page, content)?;
        pages.push(Marks {
            appearance: wrapper,
            placement: IDENTITY,
            media_box: page.media_box,
            crop_box: page.crop_box,
            rotate: i64::from(page.rotate),
        });
    }
    importer.drain()?;
    Ok(ImportedPages {
        written: importer.written,
        pages,
    })
}

/// The device colour families a converted file's output intent licenses.
///
/// ISO 19005-2 and ISO 19005-4 section 6.2.4.3: an output intent licenses `DeviceGray` whatever
/// its profile, and `DeviceRGB` or `DeviceCMYK` where its destination profile has that many
/// components — ISO 32000-2 Table 65's `/N`. Empty for a file stating no intent, whose pages
/// then use no device colour this converter let through.
pub(super) fn licensed_families(bytes: &[u8], limits: Limits) -> Vec<DeviceFamily> {
    let Ok(derived) = Document::open_with_limits(Arc::<[u8]>::from(bytes), limits) else {
        return Vec::new();
    };
    let Ok(catalog) = derived.catalog() else {
        return Vec::new();
    };
    let intents = derived.get_key(&catalog, "OutputIntents");
    let Some(intent) = intents.as_array().and_then(<[Object]>::first) else {
        return Vec::new();
    };
    let intent = derived.resolve(intent);
    let profile = intent
        .as_dict()
        .map(|intent| derived.get_key(intent, "DestOutputProfile"));
    let components = profile
        .as_ref()
        .and_then(Object::as_stream)
        .and_then(|profile| derived.get_key(&profile.dict, "N").as_integer());
    let mut families = vec![DeviceFamily::Gray];
    match components {
        Some(3) => families.push(DeviceFamily::Rgb),
        Some(4) => families.push(DeviceFamily::Cmyk),
        _ => {}
    }
    families
}

/// The identity matrix: a derived page is drawn where its own content put it.
const IDENTITY: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Copies objects of the derived file into the source's numbering, each once.
struct Importer<'a> {
    /// The derived file.
    derived: &'a Document,
    /// The source, whose numbering the copies take.
    source: &'a Document,
    /// The numbers the source resolves to nothing.
    spare: &'a mut Spare,
    /// Each derived object already given a number, with that number.
    numbers: BTreeMap<ObjectId, ObjectId>,
    /// The derived objects numbered and not yet copied.
    queue: Vec<ObjectId>,
    /// The copies.
    written: Vec<(ObjectId, Object)>,
}

impl Importer<'_> {
    /// A fresh number in the source's numbering, within [`MOST_IMPORTED_OBJECTS`].
    fn fresh(&mut self) -> Result<ObjectId, Because> {
        if self.numbers.len().saturating_add(self.written.len()) >= MOST_IMPORTED_OBJECTS {
            return Err(Because::NotBuiltYet(DERIVED_PAGES_TOO_LARGE));
        }
        self.spare
            .take(self.source)
            .ok_or(Because::NotBuiltYet(DERIVED_NO_NUMBER))
    }

    /// The source's number for one derived object, queued for copying the first time it is met.
    fn number(&mut self, at: ObjectId) -> Result<ObjectId, Because> {
        if let Some(number) = self.numbers.get(&at) {
            return Ok(*number);
        }
        let number = self.fresh()?;
        self.numbers.insert(at, number);
        self.queue.push(at);
        Ok(number)
    }

    /// One value with every reference it holds renumbered.
    fn value(&mut self, value: &Object) -> Result<Object, Because> {
        Ok(match value {
            Object::Reference(at) => Object::Reference(self.number(*at)?),
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.value(item))
                    .collect::<Result<_, _>>()?,
            ),
            Object::Dictionary(dict) => Object::Dictionary(self.dictionary(dict)?),
            Object::Stream(stream) => Object::Stream(Arc::new(Stream {
                dict: self.dictionary(&stream.dict)?,
                data: Arc::clone(&stream.data),
                decryption_failed: stream.decryption_failed,
            })),
            other => other.clone(),
        })
    }

    /// One dictionary with every reference it holds renumbered.
    fn dictionary(&mut self, dict: &Dictionary) -> Result<Dictionary, Because> {
        let mut out = Dictionary::new();
        for (key, value) in dict.iter() {
            out.insert(key.clone(), self.value(value)?);
        }
        Ok(out)
    }

    /// Copies every queued object, and every object those reach, until none is left.
    fn drain(&mut self) -> Result<(), Because> {
        while let Some(at) = self.queue.pop() {
            let object = self.derived.get(at);
            if let Some(dict) = object
                .as_dict()
                .or_else(|| object.as_stream().map(|s| &s.dict))
                && self
                    .derived
                    .get_key(dict, "Type")
                    .as_name()
                    .is_some_and(|name| matches!(name.as_bytes(), b"Page" | b"Pages"))
            {
                return Err(Because::NotBuiltYet(DERIVED_RESOURCE_NAMES_A_PAGE));
            }
            let copied = self.value(&object)?;
            let number = self.numbers[&at];
            self.written.push((number, copied));
        }
        Ok(())
    }

    /// The derived page's content and resources as one form `XObject`, in the source's numbering.
    fn page_form(&mut self, page: &pdf_model::Page) -> Result<ObjectId, Because> {
        let content = page.content(self.derived);
        let mut dict = form_dictionary(page.media_box);
        dict.insert(
            Name::new(&b"Resources"[..]),
            Object::Dictionary(self.dictionary(&page.resources)?),
        );
        // §11.6.6's page group is the form's group here: the content was composited in it.
        if let Some(group) = page.dict.get("Group") {
            let group = self.value(group)?;
            dict.insert(Name::new(&b"Group"[..]), group);
        }
        self.form(dict, &content)
    }

    /// The form a page of marks invokes: the derived page, then its annotations' appearances.
    fn wrapper(&mut self, page: &pdf_model::Page, content: ObjectId) -> Result<ObjectId, Because> {
        let mut xobjects = Dictionary::new();
        xobjects.insert(Name::new(DERIVED_PAGE), Object::Reference(content));
        let mut operators = String::from("/DerivedPage Do\n");
        let annotations = self.derived.get_key(&page.dict, "Annots");
        for (index, annotation) in annotations.as_array().into_iter().flatten().enumerate() {
            let annotation = self.derived.resolve(annotation);
            let Some(annotation) = annotation.as_dict() else {
                continue;
            };
            let Some((at, matrix)) = shown_appearance(self.derived, annotation) else {
                continue;
            };
            let name = format!("DerivedAnnotation{index}");
            xobjects.insert(
                Name::new(name.as_bytes()),
                Object::Reference(self.number(at)?),
            );
            operators.push_str("q\n");
            for value in matrix {
                // Writing into a `String` cannot fail.
                let _ = write!(operators, "{value} ");
            }
            let _ = write!(operators, "cm\n/{name} Do\nQ\n");
        }
        let mut resources = Dictionary::new();
        resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(xobjects));
        let mut dict = form_dictionary(page.media_box);
        dict.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
        self.form(dict, operators.as_bytes())
    }

    /// One form `XObject` written under a fresh number, its content compressed.
    fn form(&mut self, mut dict: Dictionary, content: &[u8]) -> Result<ObjectId, Because> {
        let data = flate_encode(content, COMPRESSION_LEVEL)
            .ok_or(Because::NotBuiltYet(DERIVED_PAGES_TOO_LARGE))?;
        dict.insert(
            Name::new(&b"Filter"[..]),
            Object::Name(Name::new(&b"FlateDecode"[..])),
        );
        dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(
                i64::try_from(data.len())
                    .map_err(|_| Because::NotBuiltYet(DERIVED_PAGES_TOO_LARGE))?,
            ),
        );
        let at = self.fresh()?;
        self.written.push((
            at,
            Object::Stream(Arc::new(Stream {
                dict,
                data: data.into(),
                decryption_failed: false,
            })),
        ));
        Ok(at)
    }
}

/// A form `XObject`'s dictionary whose `/BBox` is a page's media box.
fn form_dictionary(media_box: [f32; 4]) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XObject"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Form"[..])),
    );
    dict.insert(
        Name::new(&b"BBox"[..]),
        Object::Array(
            media_box
                .into_iter()
                .map(|value| Object::Real(f64::from(value)))
                .collect(),
        ),
    );
    dict
}

/// The normal appearance an annotation shows on a screen, and §12.5.5's matrix for it.
///
/// `None` for an annotation §12.5.3's `Hidden` or `NoView` keeps off the screen, for a `Popup`,
/// whose display is its parent's open state rather than the page's (§12.5.6.14), and for one with
/// no normal appearance stream: Table 170's `/N`, and where it is a dictionary of states, the one
/// `/AS` names.
fn shown_appearance(document: &Document, annotation: &Dictionary) -> Option<(ObjectId, [f32; 6])> {
    let flags = document.get_key(annotation, "F").as_integer().unwrap_or(0);
    if flags & (HIDDEN | NO_VIEW) != 0 {
        return None;
    }
    if document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Popup")
    {
        return None;
    }
    let appearances = document.get_key(annotation, "AP");
    let normal = appearances.as_dict()?.get("N")?;
    let at = match normal.as_reference() {
        Some(at) if document.get(at).as_stream().is_some() => at,
        _ => {
            let states = document.resolve(normal);
            let state = document.get_key(annotation, "AS");
            let state = state.as_name()?;
            states.as_dict()?.get(state.as_str()?)?.as_reference()?
        }
    };
    let stream = document.get(at);
    let stream = stream.as_stream()?;
    let matrix = pdf_model::appearance::placement(document, annotation, &stream.dict)?;
    Some((
        at,
        [matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_attachment_is_converted_to_the_least_demanding_part_its_target_admits() {
        // ISO 19005-2 section 6.8: part 1 or part 2; ISO 19005-4 section 6.9: parts 1, 2 or 4.
        for level in [Level::A, Level::B, Level::U] {
            assert_eq!(
                target_for(Target::Two(level), Placement::Attach),
                Target::Two(Level::B)
            );
        }
        assert_eq!(
            target_for(Target::Four(Flavour::Plain), Placement::Attach),
            Target::Four(Flavour::Plain)
        );
        // Pages become the archive's own, so they are held to the archive's target.
        assert_eq!(
            target_for(Target::Two(Level::A), Placement::Append),
            Target::Two(Level::A)
        );
    }
}
