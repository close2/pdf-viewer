//! Pages appended from templates (§12.7.8.3.3's `/Templates`) and the named-page files an import
//! waits for, supplies or declines.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::forms_data::Import;

use super::field_tree::widgets_by_field_name;
use super::{AppendedPage, AwaitedPage, Imported, ViewState, WantedPage};

impl ViewState {
    /// Adds §12.7.8.3.3's template pages, resolved through §12.7.7's name trees.
    ///
    /// A template page is a page **this document already holds** — §12.7.7 puts it in the
    /// catalog's `/Templates` name tree, outside the page tree so that it is not displayed until
    /// something asks for it — so adding one costs a name lookup and no page content at all.
    ///
    /// The name trees are read only when the FDF file states a page, which is never for any
    /// document anyone has opened: `CLAUDE.md`'s "nothing eager" applies, and this is the one
    /// caller either tree has.
    ///
    /// Table 252's `/Fields` are imported with the page, on the flag beside them. The entry is
    /// "the root fields that shall be imported", which is into *this* document and by §12.7.4.2's
    /// name — the same act §12.7.8.3.2 defines — so they go through
    /// [`crate::forms_data::match_fields`] exactly as Table 246's do. What decides whether they
    /// may is `/Rename`, because a template added by reference to a page this document already
    /// holds makes every one of its names a conflict with an existing field: `false` says "all
    /// fields with that name shall be updated", which is what an import already does, and `true`
    /// asks for new fields under new names, on a page [`ViewState::save`] does not write
    /// (ADR 2154). A `true` is therefore refused by name; applying it as though it were a `false`
    /// would put the template's values on the fields the flag exists to leave alone.
    ///
    /// Two refusals, each named rather than dropped, and one question. A `/TRef` naming no page
    /// in either tree is a file asking for something the document does not contain, which is the
    /// one case §12.7.7's own invariants cannot catch, and `/Rename` is the second, above. Table
    /// 253's `/F` names a template in *another file*, which is held and asked of a host rather
    /// than refused (ADR 1239).
    pub(super) fn append_templates(
        &mut self,
        document: &Document,
        data: &crate::forms_data::FormsData,
        widgets: &BTreeMap<String, Vec<ObjectId>>,
        outcome: &mut Imported,
    ) {
        // Table 246's `/EmbeddedFDFs` are files, so their pages are pages this import adds:
        // `FormsData::files` is the nesting in the order an import applies it, outermost first
        // (ADR 1185).
        let pages: Vec<&crate::forms_data::FdfPage> = data
            .files()
            .into_iter()
            .flat_map(|file| &file.pages)
            .collect();
        if pages.is_empty() {
            return;
        }
        let named = crate::named_page::NamedPages::read(document);
        let mut appearances = NamedPageAppearances::default();
        for page in pages {
            for template in &page.templates {
                let reference = &template.reference;
                if let Some(file) = &reference.file {
                    // Table 253's `/F` puts the template in a second file, which §12.7.6.4 makes
                    // a question about this machine; this crate has no filesystem and must not
                    // acquire one (ADR 1155), so the reference is held and a host is asked for
                    // the bytes. Its `/Fields` and `/Rename` travel with it, because they are
                    // applied with the page and not without it. ADR 1239.
                    outcome
                        .awaiting
                        .push(format!("the template {} is in {file}", reference.name));
                    self.awaiting.push(AwaitedPage {
                        file: file.clone(),
                        name: reference.name.clone(),
                        wanted: WantedPage::Template {
                            fields: template.fields.clone(),
                            rename: template.rename,
                        },
                    });
                    continue;
                }
                let Some(id) = named.lookup(&reference.name) else {
                    outcome.refused.push(format!(
                        "this document names no page {}, in either §12.7.7 tree",
                        reference.name
                    ));
                    continue;
                };
                self.appended.push(AppendedPage::Named(id));
                outcome.pages = outcome.pages.saturating_add(1);
                if template.fields.is_empty() {
                    continue;
                }
                if template.rename {
                    outcome.refused.push(format!(
                        "the template {}'s Table 252 /Rename is true, which asks that its fields \
                         be renamed to names this document has not got; the document is \
                         immutable here, so its {} field(s) are not imported",
                        reference.name,
                        template.fields.len()
                    ));
                    continue;
                }
                let (matched, unmatched) =
                    crate::forms_data::match_fields(&template.fields, widgets);
                let mut awaiting = Vec::new();
                for (widget, import) in &matched {
                    let mut import = import.clone();
                    appearances.resolve(document, *widget, &mut import, outcome, &mut awaiting);
                    self.reset.remove(widget);
                    self.imported.insert(*widget, import);
                }
                self.awaiting.append(&mut awaiting);
                outcome.widgets = outcome.widgets.saturating_add(matched.len());
                outcome.unmatched.extend(unmatched);
            }
        }
    }

    /// The pages §12.7.8.3.3's imported templates have added, in the order they were added.
    ///
    /// Empty for every document until an import-data action names an FDF file with a `/Pages`
    /// entry. A caller showing them puts them after the document's own, which is the only order
    /// the clause's "add … to the document" leaves available: §12.7.8.3.3 states no position.
    #[must_use]
    pub fn appended_pages(&self) -> &[AppendedPage] {
        &self.appended
    }

    /// The file §12.7.8's import is waiting for, or `None` where it is waiting for none.
    ///
    /// Table 253's `/F` as the FDF file wrote it, so that a caller with a filesystem can resolve
    /// it under whatever policy it applies to a file a *document* named — which is not this
    /// crate's decision and never was (ADR 1155). One at a time, in the order the references were
    /// met, because a person answering for two files at once is answering one question twice.
    #[must_use]
    pub fn file_awaited(&self) -> Option<&str> {
        self.awaiting.first().map(|awaited| awaited.file.as_str())
    }

    /// Every §12.7.8 named page in `file`, applied against the document a caller supplied.
    ///
    /// The second half of Table 253's `/F`, and the only half that needed anything this crate has
    /// not got. Table 249's `/APRef` becomes the widget's appearance, exactly as a page of this
    /// document does one branch over ([`crate::named_page::page_as_form`]), and Table 252's
    /// `/TRef` adds the page. What makes either safe to hold beside an immutable
    /// `pdf_syntax::Document` is that nothing of the second file crosses as a reference:
    /// `crate::forms_data`'s copy resolves every one of them, so what this state holds afterwards
    /// is a tree of direct objects and the interpreter reads it with no knowledge that a second
    /// file ever existed (ADRs 1223, 1239).
    ///
    /// `document` is the one being read and `source` is the file that arrived. Nothing is applied
    /// where the two are swapped by mistake: a name `source` does not hold is named rather than
    /// guessed.
    pub fn supply_named_pages(
        &mut self,
        document: &Document,
        file: &str,
        source: &Document,
    ) -> Imported {
        let mut outcome = Imported::default();
        let (mine, rest) = std::mem::take(&mut self.awaiting)
            .into_iter()
            .partition::<Vec<_>, _>(|awaited| awaited.file == file);
        self.awaiting = rest;
        if mine.is_empty() {
            return outcome;
        }
        self.supplied = self.supplied.saturating_add(1);
        let named = crate::named_page::NamedPages::read(source);
        let pages = crate::page::Pages::new(source);
        let widgets = widgets_by_field_name(document);
        let mut appearances = NamedPageAppearances::default();
        for awaited in mine {
            let Some(id) = named.lookup(&awaited.name) else {
                outcome.refused.push(format!(
                    "{file} names no page {}, in either §12.7.7 tree",
                    awaited.name
                ));
                continue;
            };
            match awaited.wanted {
                WantedPage::Appearance { widget, state } => self.supply_appearance(
                    &pages,
                    source,
                    SuppliedPage {
                        file,
                        name: &awaited.name,
                        id,
                    },
                    (widget, state),
                    &mut outcome,
                ),
                WantedPage::Template { fields, rename } => {
                    let Some(page) = carried_page(source, &pages, id) else {
                        outcome.refused.push(format!(
                            "the template {} of {file} is not a page this reader can copy whole: \
                             a part of its /Contents did not decode, or copying it would pass \
                             this reader's allowance for one entry that crosses a file boundary",
                            awaited.name
                        ));
                        continue;
                    };
                    // Table 31 keeps `/Annots` beside `/Contents` rather than in it, and §12.5.5
                    // draws an annotation against the page it is on — in the file that page is
                    // in. An annotation of the other document is identified by an object of that
                    // document, so it is named rather than carried (trap 5, ADR 1239).
                    if let Some(count) = annotation_count(source, id) {
                        outcome.refused.push(format!(
                            "the template {} of {file} carries {count} annotation(s), which are \
                             not part of its content stream and do not cross with the page",
                            awaited.name
                        ));
                    }
                    self.appended.push(AppendedPage::Carried(page));
                    outcome.pages = outcome.pages.saturating_add(1);
                    self.template_fields(
                        document,
                        &mut appearances,
                        &widgets,
                        (&awaited.name, &fields, rename),
                        &mut outcome,
                    );
                }
            }
        }
        self.bound_named_pages(&mut outcome);
        outcome
    }

    /// Table 249's `/APRef` made from a page of the file that arrived.
    fn supply_appearance(
        &mut self,
        pages: &crate::page::Pages<'_>,
        source: &Document,
        supplied: SuppliedPage<'_>,
        (widget, state): (ObjectId, &'static str),
        outcome: &mut Imported,
    ) {
        let SuppliedPage { file, name, id } = supplied;
        let form = crate::named_page::page_as_form(source, pages, id)
            .and_then(|form| crate::forms_data::carried(source, &form));
        let Some(form) = form else {
            outcome.refused.push(format!(
                "/APRef /{state}: the page {name} of {file} is not a page this reader can copy \
                 whole: a part of its /Contents did not decode, or copying it would pass this \
                 reader's allowance for one entry that crosses a file boundary"
            ));
            return;
        };
        if let Some(count) = annotation_count(source, id) {
            outcome.refused.push(format!(
                "/APRef /{state}: the page {name} of {file} carries {count} annotation(s), which \
                 are not part of its content stream and are not drawn into the appearance"
            ));
        }
        let Some(import) = self.imported.get_mut(&widget) else {
            return;
        };
        import
            .appearance_reference
            .retain(|(held, reference)| *held != state || reference.name != name);
        import
            .appearance
            .get_or_insert_with(Dictionary::new)
            .insert(Name::new(state.as_bytes()), form);
        outcome.widgets = outcome.widgets.saturating_add(1);
    }

    /// Table 252's `/Fields` applied with the template page they came with.
    ///
    /// The same three sentences [`Self::append_templates`] writes for a template of this
    /// document, because the flag is about the *target*'s field names and not about which file
    /// the page came out of.
    fn template_fields<'a>(
        &mut self,
        document: &'a Document,
        appearances: &mut NamedPageAppearances<'a>,
        widgets: &BTreeMap<String, Vec<ObjectId>>,
        (name, fields, rename): (&str, &[crate::forms_data::FdfField], bool),
        outcome: &mut Imported,
    ) {
        if fields.is_empty() {
            return;
        }
        if rename {
            outcome.refused.push(format!(
                "the template {name}'s Table 252 /Rename is true, which asks that its fields be \
                 renamed to names this document has not got; the document is immutable here, so \
                 its {} field(s) are not imported",
                fields.len()
            ));
            return;
        }
        let (matched, unmatched) = crate::forms_data::match_fields(fields, widgets);
        let mut awaiting = Vec::new();
        for (widget, import) in &matched {
            let mut import = import.clone();
            appearances.resolve(document, *widget, &mut import, outcome, &mut awaiting);
            self.reset.remove(widget);
            self.imported.insert(*widget, import);
        }
        self.awaiting.append(&mut awaiting);
        outcome.widgets = outcome.widgets.saturating_add(matched.len());
        outcome.unmatched.extend(unmatched);
    }

    /// [`MAX_NAMED_PAGE_FILES`] enforced, which is this program's budget and not the standard's.
    fn bound_named_pages(&mut self, outcome: &mut Imported) {
        if self.supplied < MAX_NAMED_PAGE_FILES {
            return;
        }
        for awaited in std::mem::take(&mut self.awaiting) {
            outcome.refused.push(format!(
                "the page {} is in {}, which is past this reader's limit of \
                 {MAX_NAMED_PAGE_FILES} file(s) asked for by one import",
                awaited.name, awaited.file
            ));
        }
    }

    /// What is said when a caller will not supply Table 253's `/F`.
    ///
    /// Every reference into that file, taken and worded. A refusal rather than a silence for
    /// trap 5's reason: a person importing a file is owed which button kept its own artwork and
    /// which template did not arrive.
    pub fn decline_named_pages(&mut self, file: &str) -> Vec<String> {
        let (mine, rest) = std::mem::take(&mut self.awaiting)
            .into_iter()
            .partition::<Vec<_>, _>(|awaited| awaited.file == file);
        self.awaiting = rest;
        mine.into_iter()
            .map(|awaited| {
                let what = match awaited.wanted {
                    WantedPage::Appearance { state, .. } => format!("/APRef /{state}"),
                    WantedPage::Template { .. } => "the template".to_owned(),
                };
                format!(
                    "{what}: the page {} is in {file}, which was not supplied",
                    awaited.name
                )
            })
            .collect()
    }
}

/// How many second files one §12.7.8 import will ask a host for.
///
/// Table 253 states no bound and could not: it is a fact about this machine and about a person's
/// patience rather than about any document, and every one of these costs a host question and a
/// PDF parsed. Sixteen is far past any template library an FDF file plausibly draws on and far
/// short of what a file could ask for, since every field of an imported template may name one —
/// and without a bound an imported template whose own fields name the file it came from would ask
/// forever. Past it the remaining references are refused by name (trap 38, ADR 1239).
const MAX_NAMED_PAGE_FILES: usize = 16;

/// One page of a supplied file, named three ways because the sentences need all three.
#[derive(Clone, Copy)]
struct SuppliedPage<'a> {
    /// Table 253's `/F`, as the FDF file wrote it.
    file: &'a str,
    /// Table 253's `/Name`.
    name: &'a str,
    /// Which object that name resolved to, in the file that arrived.
    id: ObjectId,
}

/// Turns §12.7.8.3.2's `/APRef` into the appearance §12.5.5 places, once per named page.
///
/// Table 249's entry is "\[a] dictionary holding references to external PDF files containing the
/// pages to use for the appearances of a push-button field", and Table 253 makes its `/F`
/// optional with a sentence about the absence: "\[i]f this entry is absent, it shall be assumed
/// that the page resides in the associated PDF file." So the entry has two branches. The one with
/// no `/F` names a page **this** document holds under §12.7.7, which is a name lookup and a page
/// converted to a form ([`crate::named_page::page_as_form`]); the one with a `/F` names a second
/// PDF file, which is §12.7.6.4's hazard and a host's question rather than this crate's.
///
/// The result is written into [`Import::appearance`], so a resolved `/APRef` and a stated `/AP`
/// reach the widget, the saved file and the display list by exactly one route — Table 249 makes
/// them the same entry of Table 170 and ranks them itself ("\[t]his entry shall be ignored if an
/// AP entry is present"), so a second route could only disagree with the first. ADR 1235.
///
/// The name trees and the page tree are read **on first use**, which for every document anybody
/// has opened is never: `CLAUDE.md`'s "nothing eager" reaches an import as much as a launch.
#[derive(Default)]
pub(super) struct NamedPageAppearances<'a> {
    /// §12.7.7's two trees and this document's page tree, read the first time a reference needs
    /// them.
    read: Option<(crate::named_page::NamedPages, crate::page::Pages<'a>)>,
    /// One form per page, because a form a hundred widgets name is one page converted once.
    forms: BTreeMap<ObjectId, Option<Object>>,
}

impl<'a> NamedPageAppearances<'a> {
    /// Replaces one import's `/APRef` with the appearance it names, and says what it could not.
    ///
    /// What is left on [`Import::appearance_reference`] afterwards is the references that did not
    /// resolve, each of which has said so (trap 5): a person importing the file is told which
    /// button kept its own artwork and why. The two lists are not the same statement — a page
    /// this document does not name is a refusal, and a page Table 253's `/F` puts in a second
    /// file is on `awaiting`, which is a question a host can still answer (ADR 1239).
    pub(super) fn resolve(
        &mut self,
        document: &'a Document,
        widget: ObjectId,
        import: &mut Import,
        outcome: &mut Imported,
        awaiting: &mut Vec<AwaitedPage>,
    ) {
        if import.appearance_reference.is_empty() {
            return;
        }
        // Destructured so that the trees and the per-page cache are two borrows rather than one:
        // a form is looked up while the trees are held.
        let Self { read, forms } = self;
        let (named, pages) = read.get_or_insert_with(|| {
            (
                crate::named_page::NamedPages::read(document),
                crate::page::Pages::new(document),
            )
        });
        let mut appearance = Dictionary::new();
        let mut unresolved = Vec::new();
        for (state, reference) in std::mem::take(&mut import.appearance_reference) {
            if let Some(file) = &reference.file {
                // §12.7.6.4's hazard: a file a *document* named. Nothing here opens one — this
                // crate has no filesystem and must not acquire one (ADR 1155) — so the reference
                // is held with the file it names and a host is asked for the bytes (ADR 1239).
                outcome.awaiting.push(format!(
                    "/APRef /{state}: the page {} is in {file}",
                    reference.name
                ));
                awaiting.push(AwaitedPage {
                    file: file.clone(),
                    name: reference.name.clone(),
                    wanted: WantedPage::Appearance { widget, state },
                });
                unresolved.push((state, reference));
                continue;
            }
            let Some(id) = named.lookup(&reference.name) else {
                outcome.refused.push(format!(
                    "/APRef /{state}: this document names no page {}, in either §12.7.7 tree",
                    reference.name
                ));
                unresolved.push((state, reference));
                continue;
            };
            let form = forms
                .entry(id)
                .or_insert_with(|| crate::named_page::page_as_form(document, pages, id))
                .clone();
            let Some(form) = form else {
                outcome.refused.push(format!(
                    "/APRef /{state}: the page {} is not a dictionary, or a part of its                      /Contents did not decode, so the appearance would be short of the marks the                      document states",
                    reference.name
                ));
                unresolved.push((state, reference));
                continue;
            };
            // Table 31 keeps `/Annots` beside `/Contents` rather than in it, and §12.5.5 draws an
            // annotation against the page it is on; a form has no counterpart, so composing them
            // in would be a composition this program decided rather than the producer.
            if let Some(count) = annotation_count(document, id) {
                outcome.refused.push(format!(
                    "/APRef /{state}: the page {} carries {count} annotation(s), which are not                      part of its content stream and are not drawn into the appearance",
                    reference.name
                ));
            }
            appearance.insert(Name::new(state.as_bytes()), form);
        }
        import.appearance_reference = unresolved;
        if !appearance.is_empty() {
            import.appearance = Some(appearance);
        }
    }
}

/// §12.7.8.3.3's template page from a second file, copied into a page this document can hold.
///
/// **Every entry is the other producer's own or a clause's**, which is the test `CLAUDE.md`'s
/// provenance line states and the same one [`crate::named_page::page_as_form`] passes one
/// subclause over (ADR 1235). The stream is §7.8.2's concatenation of that page's `/Contents` —
/// the bytes the interpreter would have read there, unchanged; the `/Resources` are the ones
/// §7.7.3.4's inheritance puts in effect, copied so that they name nothing of the other file; the
/// boxes and Table 31's `/Rotate` are what that page states. What is **not** carried is
/// `/Parent`: §7.7.3.4's inheritance runs up a tree that is in the other file, and it has already
/// been applied here.
///
/// `None` where a part of the page's `/Contents` did not decode — a page short of its marks is
/// the silent failure trap 5 is about — or where the copy would pass the allowance
/// `crate::forms_data`'s copy holds one crossing entry to.
fn carried_page(
    source: &Document,
    pages: &crate::page::Pages<'_>,
    id: ObjectId,
) -> Option<Dictionary> {
    let object = source.get(id);
    let dict = object.as_dict()?;
    // A name in the `/Pages` tree is a page of the page tree and inherits up its `/Parent`; a
    // `/Templates` name "shall have no Parent" and inherits nothing — §12.7.7's own division,
    // read the way `page_as_form` reads it.
    let page = pages
        .index_of(id)
        .and_then(|index| pages.get(index))
        .unwrap_or_else(|| pages.detached(dict));
    let (content, issues) = page.content_with_report(source);
    if !issues.is_empty() {
        return None;
    }
    let name = |bytes: &[u8]| Name::new(bytes);
    let rectangle = |edges: [f32; 4]| {
        Object::Array(
            edges
                .iter()
                .map(|edge| Object::Real(f64::from(*edge)))
                .collect(),
        )
    };
    let mut stream = Dictionary::new();
    stream.insert(
        name(b"Length"),
        Object::Integer(i64::try_from(content.len()).ok()?),
    );
    let mut out = Dictionary::new();
    out.insert(name(b"Type"), Object::Name(name(b"Page")));
    out.insert(name(b"MediaBox"), rectangle(page.media_box));
    out.insert(name(b"CropBox"), rectangle(page.crop_box));
    if page.rotate != 0 {
        out.insert(name(b"Rotate"), Object::Integer(i64::from(page.rotate)));
    }
    out.insert(
        name(b"Resources"),
        crate::forms_data::carried(source, &Object::Dictionary(page.resources.clone()))?,
    );
    // §11.4.7's page group and Table 93's group attributes are the same dictionary, and a page
    // whose marks the producer composited inside a group composites differently without it.
    if let Some(group) = dict.get("Group") {
        out.insert(name(b"Group"), crate::forms_data::carried(source, group)?);
    }
    // The bytes are written as they came out of the filters, so the stream states no `/Filter`
    // and §7.3.8.2's `/Length` is their own count.
    out.insert(
        name(b"Contents"),
        Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
            dict: stream,
            data: content.into(),
            decryption_failed: false,
        })),
    );
    Some(out)
}

/// How many annotations a page's `/Annots` holds, or `None` where it holds none.
fn annotation_count(document: &Document, page: ObjectId) -> Option<usize> {
    let object = document.get(page);
    let dict = object.as_dict()?;
    let annots = document.get_key(dict, "Annots");
    let count = annots.as_array()?.len();
    (count > 0).then_some(count)
}
