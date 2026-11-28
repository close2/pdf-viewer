//! A copy whose original nothing else draws takes the original's place, ISO 32000-2 §12.5.6.23,
//! §7.7.3.4, §7.8.3 and §8.10.1 (ADR 3861).
//!
//! A page's walk copies a form or an image it cannot prove its own (ADR 1196), and writes a
//! shading or pattern whose located data it destroyed afresh (ADR 1363): referenced more
//! than once, or reached through resources another page shares. That proof is about
//! **references**, and a reference is not a placement. Resources a page inherits from `/Pages`
//! (§7.7.3.4) or names with another page (§7.7.3.3) reach every object in them from every page
//! that shares them, so the walk copies a form one page draws and the others only inherit — and
//! the producer's stream, the covered marks included, stays in the file for pages that never draw
//! it. §12.5.6.23 asks that the content be removed, and a page that only *can* name an object
//! draws none of its marks, so keeping them for it keeps them for nobody.
//!
//! So once every page is walked, each original copied for a page is looked for among what the
//! output still draws: every page's content and every annotation's appearances, entering each
//! form drawn, through its own resources or the ones it falls back on. A redacted page's draw of
//! an object it copied is its copy's, and does not count. An original nothing still draws takes
//! the first page's edit **in its own place**, as an object the page owns does, so every
//! reference to it — the inherited resources, a structure element's `/Stm` — reaches the edited
//! object and no stream of the output holds the producer's. Where two pages copied it, the
//! second keeps its copy: the marks it removed from its own are drawn on the first page, outside
//! that page's region, so nothing it removed is held where nothing shows it.
//!
//! Every page that draws the original still draws it whole: the rule changes only what no page
//! draws, which is why it may replace the object every page's resources name — §7.8.3's names
//! resolve to the same object as before, and the drawings of every other page are unchanged.
//!
//! A shading or pattern written **directly** in resources the page inherits from a `/Pages` node
//! (§7.7.3.4) has no object of its own to take the edit, and the producer's function or sample
//! stream stays reachable from that node's entry. Where nothing else draws the entry, the object
//! holding it — the node, or the resource dictionary or category table it names by reference — is
//! restated with the page's replacement written in the entry's place and every other entry carried
//! as it was ([`DirectEntry`], ADR 3869). Every page that inherits it resolves the name to the
//! replacement, and none of them draws it, so none of their drawings changes. The same holds for
//! one written in a resource dictionary or category table that the page, or a form it draws,
//! names by reference and something else names too (§7.7.3.3, §7.8.3); and a form that states no
//! resources of its own gives its replacement to the page's (ADR 3873).
//!
//! The search errs one way only. Whatever it cannot read — a stream that does not decode, a
//! pattern, a Type 3 glyph, a soft mask's group, a resource it reaches without a `Do` naming it —
//! counts as drawing everything it reaches, and a direct entry counts as drawn wherever an equal
//! dictionary is reached; and past [`MAX_TOKENS`] every original counts as drawn. An original wrongly found drawn keeps today's copy; one wrongly found undrawn would
//! change another page, and the search is built so that it cannot be.

use std::collections::{HashMap, HashSet};

use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_syntax::lexer::{Lexer, Token};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::located::{Built, Key};
use super::{AppliedPage, MAX_DEPTH};

/// The most content-stream tokens the search reads across the whole document before it stops
/// and counts every original as drawn.
///
/// The search lexes every page's content and every form it draws once per page, which is linear
/// in the document's drawing; this bound is on that work, not on any one stream. A document past
/// it keeps every copy, which is the behaviour before the search existed, rather than being
/// searched without bound (principle 3).
const MAX_TOKENS: usize = 1 << 24;

/// The resource categories other than `/XObject` whose entries can draw a form or an image: a
/// tiling pattern's own content, a Type 3 font's glyph procedures, a graphics state's soft mask
/// group, and a shading that names none but is read the same way for safety.
const OTHER_DRAWING_CATEGORIES: [&str; 4] = ["Pattern", "Font", "ExtGState", "Shading"];

/// A shading or pattern a page's located destruction replaced (ADR 1363) that its resources state
/// directly, in an object of an ancestor `/Pages` node's rather than the page's own (§7.7.3.4).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct DirectEntry {
    /// The object that holds it: the node, or the resource dictionary or category table the
    /// node's entries name by reference.
    pub(super) holder: ObjectId,
    /// The keys from the holder's dictionary down to the entry, the entry's own name last.
    pub(super) path: Vec<Vec<u8>>,
    /// The page's key for the replacement, `Key::Direct` with no form.
    pub(super) key: Key,
    /// The entry as the producer wrote it.
    entry: Object,
}

/// What one root of the search draws in place of the producer's: the objects its page copied, and
/// the direct entries its page replaced (by index into the search's list).
#[derive(Default)]
struct Own {
    objects: HashSet<ObjectId>,
    direct: HashSet<usize>,
}

/// Lets each form or image a page copied take its original's place where nothing the output
/// still draws reaches the original (the module's rule), and gives each page the direct entries
/// whose holders it restates.
pub(super) fn adopt(
    document: &Document,
    applied: &mut [AppliedPage],
    counts: &HashMap<u32, usize>,
) {
    let candidates: HashSet<ObjectId> = applied.iter().flat_map(copied).collect();
    let mut directs: Vec<DirectEntry> = Vec::new();
    let mut own_directs: Vec<HashSet<usize>> = Vec::with_capacity(applied.len());
    for page in applied.iter() {
        let mut own = HashSet::new();
        for key in page.names.located.keys() {
            if let Some(found) = direct_entry(document, page, key, counts) {
                let index = directs
                    .iter()
                    .position(|known| known.holder == found.holder && known.path == found.path)
                    .unwrap_or_else(|| {
                        directs.push(found);
                        directs.len().saturating_sub(1)
                    });
                own.insert(index);
            }
        }
        own_directs.push(own);
    }
    if candidates.is_empty() && directs.is_empty() {
        return;
    }
    let (drawn, drawn_directs) = drawn(document, applied, (&candidates, &directs), &own_directs);
    // An object some page already replaces in place is not given a second edit in that place.
    let mut taken: HashSet<ObjectId> = applied
        .iter()
        .flat_map(|page| {
            let forms = page
                .forms
                .iter()
                .filter(|form| !form.private)
                .map(|form| form.id);
            let images = page.images.iter().filter(|image| !image.private);
            forms
                .chain(images.map(|image| image.id))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut taken_directs: HashSet<usize> = HashSet::new();
    for (page, own) in applied.iter_mut().zip(&own_directs) {
        for form in &mut page.forms {
            if form.private
                && !form.placement_copy
                && !drawn.contains(&form.id)
                && taken.insert(form.id)
            {
                form.private = false;
            }
        }
        for image in &mut page.images {
            if image.private && !drawn.contains(&image.id) && taken.insert(image.id) {
                image.private = false;
            }
        }
        let mut in_place: Vec<ObjectId> = located(page)
            .filter(|id| !drawn.contains(id) && taken.insert(*id))
            .collect();
        in_place.sort_unstable_by_key(|id| id.number);
        page.located_in_place = in_place;
        let mut direct: Vec<usize> = own
            .iter()
            .copied()
            .filter(|index| !drawn_directs.contains(index) && taken_directs.insert(*index))
            .collect();
        direct.sort_unstable();
        page.direct_in_place = direct.into_iter().map(|i| directs[i].clone()).collect();
    }
}

/// Where a page's direct shading or pattern entry `key` is written, if an object that outlives
/// the edited stream holds it: an ancestor node's resources (§7.7.3.4's inheritance), or a
/// resource dictionary or category table the stream that names it — the page, or a form with
/// resources of its own (§7.8.3) — names by reference and something else names too (§7.7.3.3).
///
/// A dictionary written directly in the page or the form is rebuilt with it and holds nothing
/// past it, and so is an object only the edited stream names: its restated resources name the
/// replacement in its place, and nothing reaches the producer's. The entry found is checked
/// against the resources the stream's names were resolved in, so a `/Parent` chain that disagrees
/// with the tree the pages were read from finds nothing rather than another dictionary.
fn direct_entry(
    document: &Document,
    page: &AppliedPage,
    key: &Key,
    counts: &HashMap<u32, usize>,
) -> Option<DirectEntry> {
    let Key::Direct {
        form,
        category,
        name,
    } = key
    else {
        return None;
    };
    // A form's names resolve in its own `/Resources`, or in the page's where it states none,
    // which is how the walk resolved them (ADR 0255).
    let stated_by_form = form.and_then(|id| {
        let object = document.get(id);
        let stream = object.as_stream()?;
        match stream.dict.get("Resources")? {
            Object::Reference(resources) => {
                let resolved = document.get(*resources);
                let resolved = resolved.as_dict()?.clone();
                Some((id, (*resources, Vec::new()), resolved))
            }
            Object::Dictionary(resources) => {
                Some((id, (id, vec![b"Resources".to_vec()]), resources.clone()))
            }
            _ => None,
        }
    });
    let (stream, (mut holder, mut path), resolved_in) = if let Some(found) = stated_by_form {
        found
    } else {
        let (node, found) = stating_node(document, page.page_id)?;
        (node, found, page.resources.clone())
    };
    let mut current = document.get(holder);
    for step in &path {
        current = match &current {
            Object::Dictionary(dict) => dict.get_by_name(&Name::new(step.as_slice()))?.clone(),
            Object::Stream(stream) => stream
                .dict
                .get_by_name(&Name::new(step.as_slice()))?
                .clone(),
            _ => return None,
        };
    }
    let Object::Dictionary(resources) = &current else {
        return None;
    };
    let table = match resources.get(category.key())? {
        Object::Reference(id) => {
            holder = *id;
            path.clear();
            document.get(*id)
        }
        direct => {
            path.push(category.key().as_bytes().to_vec());
            direct.clone()
        }
    };
    let Object::Dictionary(table) = &table else {
        return None;
    };
    let entry = table.get_by_name(&Name::new(name.as_slice()))?;
    let effective = document.get_key(&resolved_in, category.key());
    let stated = effective
        .as_dict()
        .and_then(|effective| effective.get_by_name(&Name::new(name.as_slice())));
    if matches!(entry, Object::Reference(_)) || stated != Some(entry) {
        return None;
    }
    // The stream's own dictionary is rebuilt with it; an object it names that nothing else does
    // is reached from nothing once its restated resources replace the name.
    let own = stream == page.page_id || Some(stream) == *form;
    let shared = counts.get(&holder.number).is_some_and(|count| *count > 1);
    if own && (holder == stream || !shared) {
        return None;
    }
    path.push(name.clone());
    Some(DirectEntry {
        holder,
        path,
        key: key.clone(),
        entry: entry.clone(),
    })
}

/// An object, and the keys from it down to a dictionary it holds directly.
type Place = (ObjectId, Vec<Vec<u8>>);

/// The node whose `/Resources` the page's resolve in — the page itself, or the nearest ancestor
/// stating them (§7.3.9: a null or an absent entry is inherited) — with the object holding that
/// dictionary and the keys from it down to the dictionary.
fn stating_node(document: &Document, page: ObjectId) -> Option<(ObjectId, Place)> {
    let mut node = page;
    for _ in 0..MAX_DEPTH {
        let object = document.get(node);
        let Object::Dictionary(dict) = &object else {
            return None;
        };
        match dict.get("Resources") {
            Some(Object::Reference(id)) if matches!(document.get(*id), Object::Dictionary(_)) => {
                return Some((node, (*id, Vec::new())));
            }
            Some(Object::Dictionary(_)) => {
                return Some((node, (node, vec![b"Resources".to_vec()])));
            }
            _ => {}
        }
        node = dict.get("Parent")?.as_reference()?;
    }
    None
}

/// The objects a page's resources were given copies of: what its draws of them reach instead.
/// A shading or pattern whose located data the page destroyed is one too — its streams'
/// resources name the replacement (ADR 1363) — where the entry names it by reference.
fn copied(page: &AppliedPage) -> Vec<ObjectId> {
    let forms = page
        .forms
        .iter()
        .filter(|form| form.private && !form.placement_copy)
        .map(|form| form.id);
    let images = page.images.iter().filter(|image| image.private);
    forms
        .chain(images.map(|image| image.id))
        .chain(located(page))
        .collect()
}

/// The shadings and patterns a page's streams name a replacement of by reference.
fn located(page: &AppliedPage) -> impl Iterator<Item = ObjectId> + '_ {
    page.names
        .located
        .iter()
        .filter_map(|(key, built)| match (key, built) {
            (Key::Object(id), Built::New(_)) => Some(*id),
            _ => None,
        })
}

/// Which of `candidates` and of `directs` something the output still draws reaches; every one of
/// them where the search passed its bound.
fn drawn(
    document: &Document,
    applied: &[AppliedPage],
    (candidates, directs): (&HashSet<ObjectId>, &[DirectEntry]),
    own_directs: &[HashSet<usize>],
) -> (HashSet<ObjectId>, HashSet<usize>) {
    let mut search = Search {
        document,
        candidates,
        directs,
        drawn: HashSet::new(),
        drawn_directs: HashSet::new(),
        entered: HashSet::new(),
        tokens: 0,
    };
    let everything = || (candidates.clone(), (0..directs.len()).collect());
    let copies_by_page: HashMap<ObjectId, Own> = applied
        .iter()
        .zip(own_directs)
        .map(|(page, direct)| {
            let own = Own {
                objects: copied(page).into_iter().collect(),
                direct: direct.clone(),
            };
            (page.page_id, own)
        })
        .collect();
    let none = Own::default();
    let pages = Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let own = page
            .id
            .and_then(|id| copies_by_page.get(&id))
            .unwrap_or(&none);
        let page_resources = [(*page.resources).clone()];
        let root = index.saturating_mul(2);
        search.stream(&page.content(document), &page_resources, (root, own), 0);
        // Appearances are carried as the producer wrote them, never through the page's copies,
        // so a page's own copies do not stand in for anything they draw.
        let annotations = document.get_key(&page.dict, "Annots");
        for annotation in annotations.as_array().unwrap_or_default() {
            let annotation = document.resolve(annotation);
            let Some(annotation) = annotation.as_dict() else {
                continue;
            };
            for key in ["AP", "RO"] {
                if let Some(value) = annotation.get(key) {
                    let target = (root.saturating_add(1), &none);
                    search.appearance(value, &page_resources, target, 0);
                }
            }
        }
        if let Some(thumbnail) = page.dict.get("Thumb") {
            search.reach(thumbnail, &none, 0, &mut HashSet::new());
        }
        if search.tokens > MAX_TOKENS {
            return everything();
        }
    }
    // §7.7.4's `/AP` name tree maps names to appearance streams a processor may draw.
    if let Some(catalog) = document.trailer().get("Root") {
        let catalog = document.resolve(catalog);
        if let Some(names) = catalog
            .as_dict()
            .map(|dict| document.get_key(dict, "Names"))
            && let Some(tree) = names.as_dict().and_then(|dict| dict.get("AP"))
        {
            search.reach(tree, &none, 0, &mut HashSet::new());
        }
    }
    if search.tokens > MAX_TOKENS {
        return everything();
    }
    (search.drawn, search.drawn_directs)
}

/// One search of what the output draws.
struct Search<'a> {
    document: &'a Document,
    /// The originals asked about.
    candidates: &'a HashSet<ObjectId>,
    /// The direct entries asked about.
    directs: &'a [DirectEntry],
    /// Those found drawn.
    drawn: HashSet<ObjectId>,
    /// The direct entries found drawn, by index.
    drawn_directs: HashSet<usize>,
    /// Each form already entered with resources of its own, by the root that entered it.
    entered: HashSet<(usize, ObjectId)>,
    /// Content-stream tokens read so far.
    tokens: usize,
}

impl Search<'_> {
    /// Records `id` drawn unless the root drawing it names a copy of it instead.
    fn mark(&mut self, id: ObjectId, copies: &Own) {
        if self.candidates.contains(&id) && !copies.objects.contains(&id) {
            self.drawn.insert(id);
        }
    }

    /// Records each direct entry equal to `value` drawn unless the root drawing it replaced it.
    ///
    /// Equality stands in for *the same entry*: a stream that resolves a name to the holder's
    /// entry reaches a dictionary equal to it, and an equal one reached elsewhere counting too is
    /// the error toward *drawn* the module allows.
    fn mark_direct(&mut self, value: &Object, copies: &Own) {
        for (index, direct) in self.directs.iter().enumerate() {
            if direct.entry == *value && !copies.direct.contains(&index) {
                self.drawn_directs.insert(index);
            }
        }
    }

    /// Every candidate reached from `value` by references, as drawn: what the search does not
    /// read operator by operator. A `/Parent` is not followed, since it leads up a tree rather
    /// than into what something draws.
    fn reach(&mut self, value: &Object, copies: &Own, depth: usize, seen: &mut HashSet<ObjectId>) {
        if depth >= MAX_DEPTH {
            return;
        }
        let next = depth.saturating_add(1);
        match value {
            Object::Reference(id) => {
                self.mark(*id, copies);
                if seen.insert(*id) {
                    let object = self.document.get(*id);
                    self.reach(&object, copies, next, seen);
                }
            }
            Object::Array(items) => {
                for item in items {
                    self.reach(item, copies, next, seen);
                }
            }
            Object::Dictionary(dict) => {
                self.mark_direct(value, copies);
                self.reach_entries(dict, copies, next, seen);
            }
            Object::Stream(stream) => self.reach_entries(&stream.dict, copies, next, seen),
            _ => {}
        }
    }

    /// [`Self::reach`] over a dictionary's entries.
    fn reach_entries(
        &mut self,
        dict: &Dictionary,
        copies: &Own,
        depth: usize,
        seen: &mut HashSet<ObjectId>,
    ) {
        for (key, entry) in dict.iter() {
            if key.as_bytes() != b"Parent" {
                self.reach(entry, copies, depth, seen);
            }
        }
    }

    /// An annotation's `/AP` (Table 170): a stream, or a dictionary of appearance states each
    /// of which is one; or a redaction's `/RO` form (Table 195).
    fn appearance(
        &mut self,
        value: &Object,
        fallback: &[Dictionary],
        root: (usize, &Own),
        depth: usize,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        if value.as_reference().is_some() {
            self.draw(value, fallback, root, depth);
            let resolved = self.document.resolve(value);
            if let Some(dict) = resolved.as_dict() {
                for (_key, entry) in dict.iter() {
                    self.appearance(entry, fallback, root, depth.saturating_add(1));
                }
            }
        } else if let Some(dict) = value.as_dict() {
            for (_key, entry) in dict.iter() {
                self.appearance(entry, fallback, root, depth.saturating_add(1));
            }
        }
    }

    /// One `Do` of a resource entry: the entry recorded, and a form entered or an image's masks
    /// and alternates reached.
    fn draw(
        &mut self,
        entry: &Object,
        enclosing: &[Dictionary],
        (root, copies): (usize, &Own),
        depth: usize,
    ) {
        let id = entry.as_reference();
        if let Some(id) = id {
            self.mark(id, copies);
        }
        let object = self.document.resolve(entry);
        let Some(stream) = object.as_stream() else {
            return;
        };
        let subtype = self.document.get_key(&stream.dict, "Subtype");
        if subtype.as_name().map(Name::as_bytes) != Some(b"Form".as_slice()) {
            self.reach_entries(&stream.dict, copies, 0, &mut HashSet::new());
            return;
        }
        // Table 93's `/Resources`, or the enclosing stream's where it states none (ADR 0255).
        let own = self.document.get_key(&stream.dict, "Resources");
        let resources: Vec<Dictionary> = match own.as_dict() {
            Some(dict) => vec![dict.clone()],
            None => enclosing.to_vec(),
        };
        if own.as_dict().is_some()
            && let Some(id) = id
            && !self.entered.insert((root, id))
        {
            return;
        }
        match self.document.decoded_stream_data(stream) {
            Some(content) => self.stream(&content, &resources, (root, copies), depth),
            None => {
                for dict in &resources {
                    self.reach_entries(dict, copies, 0, &mut HashSet::new());
                }
            }
        }
    }

    /// Reads a content stream for what it draws: each `Do` resolved in `resources` and followed,
    /// each name another operator gives looked up in [`OTHER_DRAWING_CATEGORIES`] and reached.
    fn stream(
        &mut self,
        content: &[u8],
        resources: &[Dictionary],
        root: (usize, &Own),
        depth: usize,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        let mut lexer = Lexer::at(content, 0);
        let mut names: Vec<Vec<u8>> = Vec::new();
        loop {
            if self.tokens > MAX_TOKENS {
                return;
            }
            lexer.skip_whitespace();
            let Some(token) = lexer.next_token() else {
                break;
            };
            self.tokens = self.tokens.saturating_add(1);
            match token {
                Token::Name(bytes) => names.push(bytes),
                Token::Keyword(b"BI") => {
                    // §8.9.7: the image's data is not tokens, and is stepped over whole.
                    let resources = resources.first().cloned().unwrap_or_default();
                    let at = lexer.position();
                    let scan =
                        pdf_model::inline_image::scan(self.document, content, at, &resources, true);
                    lexer.seek(scan.resume.max(at));
                    names.clear();
                }
                Token::Keyword(b"Do") => {
                    if let Some(name) = names.last() {
                        let name = Name::new(name.as_slice());
                        for dict in resources {
                            let table = self.document.get_key(dict, "XObject");
                            if let Some(entry) = table.as_dict().and_then(|t| t.get_by_name(&name))
                            {
                                let entry = entry.clone();
                                self.draw(&entry, resources, root, depth.saturating_add(1));
                            }
                        }
                    }
                    names.clear();
                }
                Token::Keyword(_) => {
                    for name in names.drain(..) {
                        let name = Name::new(name.as_slice());
                        for dict in resources {
                            for category in OTHER_DRAWING_CATEGORIES {
                                let table = self.document.get_key(dict, category);
                                if let Some(entry) =
                                    table.as_dict().and_then(|t| t.get_by_name(&name))
                                {
                                    let entry = entry.clone();
                                    self.reach(&entry, root.1, 0, &mut HashSet::new());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
