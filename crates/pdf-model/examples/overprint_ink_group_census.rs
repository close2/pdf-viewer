//! How many crawled pages actually reach §11.7.4.3's special overprinting blend mode.
//!
//! ADR 1158 section 4 left one number unmeasured: whether a producer that enables overprinting
//! inside a `DeviceCMYK` group is common enough for `render-raster`'s and `render-gpu`'s
//! by-name refusal to cost coverage. The population is the conjunction of three things —
//! §11.7.2's subtractive compositing space, §8.6.7's overprint parameter with an overprint mode
//! of 1, and a directly-specified `DeviceCMYK` colour one of whose tints is zero — and it is
//! counted here from the interpreter's own verdict rather than from a second copy of the rule.
//!
//! # Where each column comes from
//!
//! - **The mode was built**: `DisplayList::overprints()`, which is set at the one call
//!   `content::overprint` makes when all of its conditions hold. Nothing here re-derives
//!   `/OP`, `/OPM` or the zero tint, which is `doc/traps/`'s rule that a census whose predicate
//!   is a second copy of the rule measures the copy.
//! - **Marks under the mode**: every command whose `blend()` is `BlendMode::Overprint`, over
//!   both halves of §11.4.7's page pair, inside every group, inside every `Shaped` command's
//!   object and inside every §11.6.5.1 soft-mask group's elements.
//! - **The verdict with no mark**, which is this example's own check. `DisplayList::overprints`
//!   is settled against the commands the finished list holds (ADR 1181), so this column is zero
//!   and a page in it is a command shape this walk does not reach or a route into the list the
//!   settling does not follow. It matters beyond this census: `render-raster` and `render-gpu`
//!   refuse a whole page by name on that flag, so a page in this column would be refused for a
//!   mark that is not on it.
//! - **The shape of the kept set**, which is what a scene vocabulary would have to express. The
//!   mode is Porter-Duff destination-over in the channels it keeps and source-over in the rest
//!   (ADR 1182), so a mark whose set is all three channels or none of them is one existing
//!   compositing operator and a mark whose set is a proper subset is a per-channel choice
//!   between two. Counted apart, because the two are different asks.
//! - **Under a non-Normal blend mode**: §11.7.4.3's last paragraph builds a non-isolated,
//!   non-knockout group around such an object and composites it under the mode in the
//!   graphics state, so that group — non-Normal blend, an overprinting command directly
//!   inside it — *is* the answer, read off the display list the interpreter produced.
//! - **Reported instead**: `Unsupported::Overprint`, whose two sentences are §11.7.4.3's
//!   knockout case and §11.7.4.4's first bullet's. Matched on their opening clause numbers,
//!   which is the report's own wording.
//!
//! # The pre-filter, and why it cannot lose a page
//!
//! Interpreting every page of the crawl is a walk nobody needs here: §8.6.7's overprint
//! parameters are Table 58's `/OP` and `/op` and an `ExtGState` dictionary is the only place
//! either can be set, so a document no object of which carries one of those two keys cannot
//! reach the mode on any page. Every other document is interpreted page by page. The filter is
//! an over-approximation on purpose — it asks for the *key*, not for the value or for the
//! dictionary's type — so it can admit a document that reaches nothing and cannot exclude one
//! that does.
//!
//! ```sh
//! cargo build --profile gates -p pdf-model --example overprint_ink_group_census
//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 --tree 12 -- \
//!     <target-dir>/gates/examples/overprint_ink_group_census @paths.txt
//! ```
//!
//! An argument of the form `@paths.txt` names a file holding one path per line.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_render::{BlendMode, Command, DisplayList};
use pdf_syntax::{Document, Object, ObjectId};

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

/// What one page's display list and reports said about §11.7.4.3.
#[derive(Default)]
struct Page {
    /// How many commands are painted under the special overprinting blend mode.
    marks: usize,
    /// How many of §11.7.4.3's implicit groups stand around one: an overprinting mark whose
    /// surrounding blend mode is not Normal.
    under_non_normal: usize,
    /// How many of the marks stand inside one of those groups.
    marks_under_non_normal: usize,
    /// Every non-Normal blend mode one of those groups composites under, by name.
    modes: BTreeMap<String, usize>,
    /// Marks whose kept set is all three channels of this raster, which is destination-over.
    marks_kept_all: usize,
    /// Marks whose kept set is empty, which is source-over — the same arithmetic as Normal.
    marks_kept_none: usize,
    /// Marks whose kept set is a proper subset, which is a per-channel choice between the two.
    marks_kept_mixed: usize,
    /// How many `Unsupported::Overprint` reports §11.7.4.3's knockout case produced.
    refused_knockout: usize,
    /// How many §11.7.4.4's first bullet produced.
    refused_first_bullet: usize,
}

impl Page {
    /// Whether anything on this page reached the mode or was reported for it.
    fn touched(&self) -> bool {
        self.marks > 0 || self.refused_knockout > 0 || self.refused_first_bullet > 0
    }
}

/// What one document said.
#[derive(Default)]
struct Says {
    /// Documents that opened.
    opened: usize,
    /// Documents no object of which carries Table 58's `/OP` or `/op`.
    without_the_parameter: usize,
    /// Documents that carry one and were therefore interpreted.
    interpreted: usize,
    /// Pages interpreted.
    pages: usize,
    /// Pages the mode was built on.
    pages_overprinting: usize,
    /// Pages whose verdict is `overprints()` and on which this walk finds no mark.
    ///
    /// A self-check rather than a finding: the verdict is the interpreter's and the marks are
    /// this example's reading of the display list, so a page in this column is a command shape
    /// the walk does not reach and every other column on it is short. It should be zero.
    pages_verdict_without_a_mark: usize,
    /// Documents with at least one such page.
    documents_overprinting: usize,
    /// Pages carrying at least one of §11.7.4.3's implicit groups.
    pages_under_non_normal: usize,
    /// Documents with at least one such page.
    documents_under_non_normal: usize,
    /// Pages carrying an `Unsupported::Overprint`.
    pages_reported: usize,
    /// Marks painted under the mode.
    marks: usize,
    /// Marks inside one of §11.7.4.3's implicit groups.
    marks_under_non_normal: usize,
    /// Marks whose kept set is all three channels of the raster they are drawn in.
    marks_kept_all: usize,
    /// Marks whose kept set is empty.
    marks_kept_none: usize,
    /// Marks whose kept set is a proper subset of the three.
    marks_kept_mixed: usize,
    /// Pages painting under the mode with no mark whose kept set is a proper subset.
    pages_uniform_only: usize,
    /// Documents all of whose overprinting pages are in that column.
    documents_uniform_only: usize,
    /// Every non-Normal blend mode such a group composites under, by name.
    modes: BTreeMap<String, usize>,
    /// The two report sentences, counted apart.
    refused_knockout: usize,
    /// §11.7.4.4's first bullet's.
    refused_first_bullet: usize,
    /// One line per document that reached the mode, for a round that has to read ten of them.
    witnesses: Vec<String>,
}

impl Says {
    /// Folds one document's answers into a running total.
    fn absorb(&mut self, other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.without_the_parameter = self
            .without_the_parameter
            .saturating_add(other.without_the_parameter);
        self.interpreted = self.interpreted.saturating_add(other.interpreted);
        self.pages = self.pages.saturating_add(other.pages);
        self.pages_overprinting = self
            .pages_overprinting
            .saturating_add(other.pages_overprinting);
        self.pages_verdict_without_a_mark = self
            .pages_verdict_without_a_mark
            .saturating_add(other.pages_verdict_without_a_mark);
        self.documents_overprinting = self
            .documents_overprinting
            .saturating_add(other.documents_overprinting);
        self.pages_under_non_normal = self
            .pages_under_non_normal
            .saturating_add(other.pages_under_non_normal);
        self.documents_under_non_normal = self
            .documents_under_non_normal
            .saturating_add(other.documents_under_non_normal);
        self.pages_reported = self.pages_reported.saturating_add(other.pages_reported);
        self.marks = self.marks.saturating_add(other.marks);
        self.marks_under_non_normal = self
            .marks_under_non_normal
            .saturating_add(other.marks_under_non_normal);
        self.marks_kept_all = self.marks_kept_all.saturating_add(other.marks_kept_all);
        self.marks_kept_none = self.marks_kept_none.saturating_add(other.marks_kept_none);
        self.marks_kept_mixed = self.marks_kept_mixed.saturating_add(other.marks_kept_mixed);
        self.pages_uniform_only = self
            .pages_uniform_only
            .saturating_add(other.pages_uniform_only);
        self.documents_uniform_only = self
            .documents_uniform_only
            .saturating_add(other.documents_uniform_only);
        for (mode, count) in other.modes {
            let slot = self.modes.entry(mode).or_default();
            *slot = slot.saturating_add(count);
        }
        self.refused_knockout = self.refused_knockout.saturating_add(other.refused_knockout);
        self.refused_first_bullet = self
            .refused_first_bullet
            .saturating_add(other.refused_first_bullet);
        self.witnesses.extend(other.witnesses);
    }
}

/// Whether any object in this document carries Table 58's `/OP` or `/op`.
///
/// See the module comment: §8.6.7's parameters are set only through an `ExtGState`, so a
/// document without either key reaches nothing this census counts.
fn states_an_overprint_parameter(document: &Document) -> bool {
    document.xref().object_numbers().any(|number| {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let dict = match &object {
            Object::Dictionary(dict) => Some(dict),
            Object::Stream(stream) => Some(&stream.dict),
            _ => None,
        };
        dict.is_some_and(|dict| dict.get("OP").is_some() || dict.get("op").is_some())
    })
}

/// Walks one command list, counting the marks under the mode and the groups around them.
fn walk(commands: &[Command], inside_non_normal: Option<&BlendMode>, page: &mut Page) {
    for command in commands {
        // §11.6.4.2's shape carries the object inside it, so a mark under the mode can stand
        // one level below where a walk over a command list looks. The object is walked in the
        // command's place and the shape is not: the two mark the same region, and the shape is
        // the object with its opacity removed rather than a second mark. Walked instead of
        // counted here, because `Command::blend` already answers a `Shaped` with its object's
        // mode and counting both would count one mark twice.
        if let Command::Shaped { object, .. } = command {
            walk(std::slice::from_ref(object), inside_non_normal, page);
            continue;
        }
        if let BlendMode::Overprint(overprint) = command.blend() {
            page.marks = page.marks.saturating_add(1);
            // Which of the three channels this mark leaves to the backdrop, which is the whole
            // of what a scene vocabulary would have to carry: all three is destination-over,
            // none is source-over, and anything between is a per-channel choice (ADR 1182).
            match overprint.kept().iter().filter(|channel| **channel).count() {
                3 => page.marks_kept_all = page.marks_kept_all.saturating_add(1),
                0 => page.marks_kept_none = page.marks_kept_none.saturating_add(1),
                _ => page.marks_kept_mixed = page.marks_kept_mixed.saturating_add(1),
            }
            if let Some(outer) = inside_non_normal {
                page.marks_under_non_normal = page.marks_under_non_normal.saturating_add(1);
                let slot = page.modes.entry(format!("{outer:?}")).or_default();
                *slot = slot.saturating_add(1);
            }
        }
        if let Command::Group {
            commands, blend, ..
        } = command
        {
            // §11.7.4.3's implicit group: a non-Normal blend mode with an overprinting mark
            // directly inside it. Counted once per group, which is once per object the clause
            // wrapped. A `Shaped` element is asked through its own object, for the reason
            // above.
            let non_normal = (*blend != BlendMode::Normal).then_some(blend);
            if non_normal.is_some() && commands.iter().any(overprints_here) {
                page.under_non_normal = page.under_non_normal.saturating_add(1);
            }
            walk(commands, non_normal.or(inside_non_normal), page);
        }
    }
}

/// Whether this command is itself painted under the special overprinting blend mode.
///
/// A `Shaped` is asked about its object, which is what `Command::blend` already does; this
/// exists so that the group test above and the mark count answer the same question.
fn overprints_here(command: &Command) -> bool {
    matches!(command.blend(), BlendMode::Overprint(_))
}

/// Every command list one display list holds.
///
/// Three, not one: §11.4.7's page pair is two lists of the same page, and §11.6.5.1's soft
/// masks are groups whose elements are stored beside the commands rather than among them. A
/// mark painted under the special mode can stand in any of the three, and the verdict
/// `DisplayList::overprints()` is one flag over all of them — which is what the
/// verdict-without-a-mark column checks this against.
fn walk_both(list: &DisplayList, page: &mut Page) {
    walk(list.commands(), None, page);
    for index in 0..list.soft_mask_count() {
        if let Some(mask) = list.soft_mask(pdf_render::SoftMaskId::new(
            u32::try_from(index).unwrap_or(u32::MAX),
        )) {
            walk(&mask.commands, None, page);
        }
    }
    if let Some(black) = list.black() {
        walk_both(black, page);
    }
}

/// Interprets one document's pages.
fn examine(path: &str) -> Says {
    let mut says = Says::default();
    let Ok(bytes) = std::fs::read(path) else {
        return says;
    };
    let Ok(document) = Document::open(bytes) else {
        return says;
    };
    says.opened = 1;
    if !states_an_overprint_parameter(&document) {
        says.without_the_parameter = 1;
        return says;
    }
    says.interpreted = 1;
    let name = std::path::Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |file| file.to_string_lossy().into_owned(),
    );

    let pages = pdf_model::Pages::new(&document);
    let mut reached = Vec::new();
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        says.pages = says.pages.saturating_add(1);
        let interpreted = pdf_model::interpret(&document, &page);
        let mut found = Page::default();
        let verdict = interpreted.display_list.overprints();
        if verdict {
            walk_both(&interpreted.display_list, &mut found);
            if found.marks == 0 {
                says.pages_verdict_without_a_mark =
                    says.pages_verdict_without_a_mark.saturating_add(1);
            }
        }
        for report in &interpreted.unsupported {
            let pdf_model::Unsupported::Overprint { detail } = report else {
                continue;
            };
            // The report's own opening words, which are the two constructions apart.
            if detail.starts_with("\u{a7}11.7.4.4") {
                found.refused_first_bullet = found.refused_first_bullet.saturating_add(1);
            } else {
                found.refused_knockout = found.refused_knockout.saturating_add(1);
            }
        }
        if !found.touched() && !verdict {
            continue;
        }
        says.pages_overprinting = says
            .pages_overprinting
            .saturating_add(usize::from(found.marks > 0));
        says.pages_under_non_normal = says
            .pages_under_non_normal
            .saturating_add(usize::from(found.under_non_normal > 0));
        says.pages_reported = says.pages_reported.saturating_add(usize::from(
            found.refused_knockout > 0 || found.refused_first_bullet > 0,
        ));
        says.marks = says.marks.saturating_add(found.marks);
        says.marks_under_non_normal = says
            .marks_under_non_normal
            .saturating_add(found.marks_under_non_normal);
        says.marks_kept_all = says.marks_kept_all.saturating_add(found.marks_kept_all);
        says.marks_kept_none = says.marks_kept_none.saturating_add(found.marks_kept_none);
        says.marks_kept_mixed = says.marks_kept_mixed.saturating_add(found.marks_kept_mixed);
        says.pages_uniform_only = says
            .pages_uniform_only
            .saturating_add(usize::from(found.marks > 0 && found.marks_kept_mixed == 0));
        for (mode, count) in found.modes {
            let slot = says.modes.entry(mode).or_default();
            *slot = slot.saturating_add(count);
        }
        says.refused_knockout = says.refused_knockout.saturating_add(found.refused_knockout);
        says.refused_first_bullet = says
            .refused_first_bullet
            .saturating_add(found.refused_first_bullet);
        reached.push(format!(
            "page {} ({} mark(s), {} implicit group(s), {} report(s))",
            index.saturating_add(1),
            found.marks,
            found.under_non_normal,
            found
                .refused_knockout
                .saturating_add(found.refused_first_bullet),
        ));
    }
    if !reached.is_empty() {
        says.documents_overprinting = 1;
        says.documents_under_non_normal = usize::from(says.pages_under_non_normal > 0);
        says.documents_uniform_only = usize::from(says.marks_kept_mixed == 0);
        says.witnesses
            .push(format!("{name}: {}", reached.join("; ")));
    }
    says
}

/// The paths to walk: the arguments, and the lines of any argument beginning with `@`.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => println!("{list}: {error}"),
            },
            None => out.push(argument),
        }
    }
    out
}

fn main() {
    let paths = paths();
    eprintln!("{} PDF(s) in the population", paths.len());

    let says =
        paths
            .par_iter()
            .map(|path| examine(path))
            .reduce(Says::default, |mut total, one| {
                total.absorb(one);
                total
            });

    // What it matched, before the count that summarises it.
    for witness in &says.witnesses {
        println!("{witness}");
    }

    println!(
        "\n{} path(s), {} opened, {} state no /OP or /op anywhere, {} interpreted over {} page(s)",
        paths.len(),
        says.opened,
        says.without_the_parameter,
        says.interpreted,
        says.pages
    );
    println!(
        "  {} document(s) and {} page(s) paint under \u{a7}11.7.4.3's special overprinting blend \
         mode, {} mark(s) in all",
        says.documents_overprinting, says.pages_overprinting, says.marks
    );
    println!(
        "  {} page(s) whose verdict is overprints() and on which this walk finds no mark, which \
         is this example's own check and not a finding",
        says.pages_verdict_without_a_mark
    );
    println!(
        "  {} document(s) and {} page(s) do it under a non-Normal blend mode, {} mark(s) inside \
         \u{a7}11.7.4.3's implicit group",
        says.documents_under_non_normal, says.pages_under_non_normal, says.marks_under_non_normal
    );
    for (mode, count) in &says.modes {
        println!("      {count:>6}  under {mode}");
    }
    println!(
        "  of those mark(s), {} keep all three channels (destination-over), {} keep none \
         (source-over), {} keep a proper subset (a per-channel choice); {} page(s) and {} \
         document(s) state no proper subset at all",
        says.marks_kept_all,
        says.marks_kept_none,
        says.marks_kept_mixed,
        says.pages_uniform_only,
        says.documents_uniform_only
    );
    println!(
        "  {} page(s) carry an Unsupported::Overprint: {} for \u{a7}11.7.4.3's knockout case, {} \
         for \u{a7}11.7.4.4's first bullet",
        says.pages_reported, says.refused_knockout, says.refused_first_bullet
    );
}
