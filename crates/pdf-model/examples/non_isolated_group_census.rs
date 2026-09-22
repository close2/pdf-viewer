//! Which pages state ISO 32000-2 §11.4.4's result step — a non-isolated group composited
//! under a blend mode of its own — read off the display list the interpreter produced.
//!
//! §11.4.4's NOTE 3 is the condition, and it is about the group's `Do` rather than about the
//! file's dictionary: under Normal the backdrop composited in cancels against the backdrop
//! composited back out, and the two steps collapse to one interpolation; under any other mode
//! they do not, and `render-cpu` runs the group's elements a second time onto transparency for
//! Table 140's group alpha before performing NOTE 3's removal (`CpuRasterizer::
//! remove_the_backdrop`, ADR 1107). That second run is the only cost this construction has, so
//! the population that pays it is the population this census counts.
//!
//! # Why the condition is the rasteriser's and not the clause's
//!
//! A census derived from the clause is a census of a different population from one derived
//! from the program: the code has conditions the clause does not, and the reverse. The
//! predicate here is the exact conjunction `CpuRasterizer::group_buffer` tests before it calls
//! for the second run — no blending colour space of its own, not isolated, not a knockout
//! group, and a blend mode at the `Do` that is not Normal — so a change to the rule moves the
//! number rather than leaving a second copy of the rule behind to measure.
//!
//! **The file is not the only thing that states such a group.** §11.7.4.3's last paragraph has
//! the *interpreter* build one around an object painted while overprinting is enabled under a
//! non-Normal mode, and §11.7.4.4's first bullet builds another
//! (`content::overprint::non_isolated_group`, ADR 1170). Those commands are in the display list
//! beside the file's own, and a count taken from the file's dictionaries cannot see them —
//! which is the whole reason this walk reads commands. They are counted apart: a group holding
//! a mark painted under §11.7.4.3's special overprinting blend mode is one of the two the
//! clause built, and every other one is the file's.
//!
//! # The pre-filter, and why it cannot lose a page
//!
//! Interpreting every page of a crawl is a walk nobody needs here. The mode at the `Do` is the
//! current blend mode in the graphics state, and §11.3.5 says where that parameter comes from:
//! the `/BM` entry of a graphics state parameter dictionary, which §8.4.5 is about and Table 58
//! states, is the only thing that sets it.
//!
//! So a document no object of which carries Table 58's `/BM` cannot reach a non-Normal mode on
//! any page, and therefore cannot reach this population — whether the group is the file's or
//! one the interpreter built, because both take the mode from the same parameter. The filter
//! asks for the *key* and not for its value, so it admits documents that reach nothing and
//! excludes none that does.
//!
//! # Running it
//!
//! ```sh
//! cargo build --profile gates -p pdf-model --example non_isolated_group_census
//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 --tree 12 -- \
//!     <target-dir>/gates/examples/non_isolated_group_census @paths.txt
//! ```
//!
//! An argument of the form `@paths.txt` names a file holding one path per line. Only each
//! document's **first** page is interpreted, which is the denominator §11.4.4's ledger row
//! states; `--pages N` walks more.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_render::{BlendMode, Command, DisplayList};
use pdf_syntax::{Document, Object, ObjectId};

/// What one page's groups came to.
#[derive(Default)]
struct Page {
    /// Every `Command::Group` the page's lists hold, at any depth.
    groups: usize,
    /// Of them, those with `isolated` false.
    non_isolated: usize,
    /// Of those, the ones §11.4.4's result step is performed for: `CpuRasterizer::
    /// group_buffer`'s own conjunction, which is this census's predicate.
    second_run: usize,
    /// Of those, the ones at least one of whose direct elements blends.
    ///
    /// §11.4.4's NOTE 3 makes the removal exact and the backdrop cancel where every element
    /// blends Normal, so this is the sub-population where the second run changes a pixel
    /// rather than costing one; ADR 1107's own sentence names it as a condition.
    second_run_blending_element: usize,
    /// Of the second-run groups, the ones §11.7.4's two constructions built — a group holding
    /// a mark painted under §11.7.4.3's special overprinting blend mode (ADR 1170).
    synthesised: usize,
    /// Of the second-run groups, the ones the file stated: every one that is not
    /// [`Self::synthesised`].
    file_stated: usize,
    /// Every mode such a group composites under, by name.
    modes: BTreeMap<String, usize>,
    /// Non-isolated **knockout** groups, which take ADR 0327's construction instead
    /// (`CpuRasterizer::knockout_on_backdrop`) and pay no second run.
    ///
    /// Printed because a predicate's exclusions are part of it: a reader of the second-run
    /// count needs to see what the conjunction set aside and how large it was.
    non_isolated_knockout: usize,
    /// Non-isolated groups composited under Normal, where NOTE 3's cancellation collapses the
    /// two steps to one interpolation. The other exclusion.
    non_isolated_normal: usize,
    /// Non-isolated groups carrying a blending colour space of their own.
    ///
    /// This census's self-check rather than a finding, and it should stay zero: `pdf-model`
    /// emits `blending` for an isolated group alone, because §11.6.6 gives a `/CS` effect for
    /// isolated groups alone. A page in this column is a group whose two flags disagree with
    /// that guarantee, and `CpuRasterizer::group_buffer` would composite it in its own space
    /// and never reach the removal.
    non_isolated_with_a_space: usize,
}

impl Page {
    /// Whether this page holds anything the census is about.
    fn touched(&self) -> bool {
        self.non_isolated > 0
    }
}

/// What one document said.
#[derive(Default)]
struct Says {
    /// Documents that opened.
    opened: usize,
    /// Documents no object of which carries Table 58's `/BM`.
    without_the_parameter: usize,
    /// Documents that carry one and were therefore interpreted.
    interpreted: usize,
    /// Pages interpreted.
    pages: usize,
    /// Groups seen.
    groups: usize,
    /// Non-isolated groups seen.
    non_isolated: usize,
    /// Groups §11.4.4's result step is performed for.
    second_run: usize,
    /// Of them, the ones holding an element that blends.
    second_run_blending_element: usize,
    /// Of them, the ones §11.7.4's constructions built.
    synthesised: usize,
    /// Of them, the ones the file stated.
    file_stated: usize,
    /// Pages holding at least one second-run group.
    pages_second_run: usize,
    /// Pages holding at least one built by §11.7.4.
    pages_synthesised: usize,
    /// Pages holding at least one the file stated.
    pages_file_stated: usize,
    /// Documents holding at least one second-run group on a walked page.
    documents_second_run: usize,
    /// Every mode such a group composites under, by name.
    modes: BTreeMap<String, usize>,
    /// Non-isolated knockout groups — the first exclusion.
    non_isolated_knockout: usize,
    /// Non-isolated groups under Normal — the second.
    non_isolated_normal: usize,
    /// Non-isolated groups with a blending space, which is the self-check.
    non_isolated_with_a_space: usize,
    /// One line per document that reached the population, for a round that has to read ten.
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
        self.groups = self.groups.saturating_add(other.groups);
        self.non_isolated = self.non_isolated.saturating_add(other.non_isolated);
        self.second_run = self.second_run.saturating_add(other.second_run);
        self.second_run_blending_element = self
            .second_run_blending_element
            .saturating_add(other.second_run_blending_element);
        self.synthesised = self.synthesised.saturating_add(other.synthesised);
        self.file_stated = self.file_stated.saturating_add(other.file_stated);
        self.pages_second_run = self.pages_second_run.saturating_add(other.pages_second_run);
        self.pages_synthesised = self
            .pages_synthesised
            .saturating_add(other.pages_synthesised);
        self.pages_file_stated = self
            .pages_file_stated
            .saturating_add(other.pages_file_stated);
        self.documents_second_run = self
            .documents_second_run
            .saturating_add(other.documents_second_run);
        for (mode, count) in other.modes {
            let slot = self.modes.entry(mode).or_default();
            *slot = slot.saturating_add(count);
        }
        self.non_isolated_knockout = self
            .non_isolated_knockout
            .saturating_add(other.non_isolated_knockout);
        self.non_isolated_normal = self
            .non_isolated_normal
            .saturating_add(other.non_isolated_normal);
        self.non_isolated_with_a_space = self
            .non_isolated_with_a_space
            .saturating_add(other.non_isolated_with_a_space);
        self.witnesses.extend(other.witnesses);
    }
}

/// Whether any object in this document carries Table 58's `/BM`.
///
/// See the module comment: §11.3.5 makes that entry the only source of a non-Normal current
/// blend mode, so a document without it reaches nothing this census counts.
///
/// **The search is nested and that is what the calibration bought.** A `/BM` lives in an
/// `ExtGState`, which a producer may write as an indirect object *or* as a direct dictionary
/// inside a `/Resources` dictionary two levels down — and a filter reading only each object's
/// own top level excluded the planted fixture, which is exactly the false zero trap 13 exists
/// to catch. References are not followed, because every indirect object is asked in turn
/// anyway, and that is also what keeps a cyclic document finite here.
fn states_a_blend_mode(document: &Document) -> bool {
    document.xref().object_numbers().any(|number| {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        names_a_blend_mode(&object)
    })
}

/// Whether this object, or anything directly inside it, carries Table 58's `/BM`.
fn names_a_blend_mode(object: &Object) -> bool {
    match object {
        Object::Dictionary(dict) => {
            dict.get("BM").is_some() || dict.iter().any(|(_, value)| names_a_blend_mode(value))
        }
        Object::Stream(stream) => {
            stream.dict.get("BM").is_some()
                || stream
                    .dict
                    .iter()
                    .any(|(_, value)| names_a_blend_mode(value))
        }
        Object::Array(items) => items.iter().any(names_a_blend_mode),
        _ => false,
    }
}

/// Whether this command is painted under §11.7.4.3's special overprinting blend mode.
///
/// The interpreter builds that mode at one place and nothing else in the display list carries
/// it, so a group holding such a mark directly is one of §11.7.4's two implicit groups rather
/// than one the file's `/Group` dictionary stated (ADR 1170).
fn overprints_here(command: &Command) -> bool {
    matches!(command.blend(), BlendMode::Overprint(_))
}

/// Whether this command is painted under any mode but Normal.
fn blends_here(command: &Command) -> bool {
    command.blend() != BlendMode::Normal
}

/// Walks one command list, counting every group in it and then its elements.
fn walk(commands: &[Command], page: &mut Page) {
    for command in commands {
        // §11.6.4.2's shape carries the object inside it, so a group can stand one level below
        // where a walk over a command list looks. The object is walked in the command's place.
        if let Command::Shaped { object, .. } = command {
            walk(std::slice::from_ref(object), page);
            continue;
        }
        let Command::Group {
            commands,
            blend,
            isolated,
            knockout,
            blending,
            ..
        } = command
        else {
            continue;
        };
        page.groups = page.groups.saturating_add(1);
        if !*isolated {
            page.non_isolated = page.non_isolated.saturating_add(1);
            if blending.is_some() {
                page.non_isolated_with_a_space = page.non_isolated_with_a_space.saturating_add(1);
            } else if *knockout {
                page.non_isolated_knockout = page.non_isolated_knockout.saturating_add(1);
            } else if *blend == BlendMode::Normal {
                page.non_isolated_normal = page.non_isolated_normal.saturating_add(1);
            } else {
                page.second_run = page.second_run.saturating_add(1);
                if commands.iter().any(blends_here) {
                    page.second_run_blending_element =
                        page.second_run_blending_element.saturating_add(1);
                }
                if commands.iter().any(overprints_here) {
                    page.synthesised = page.synthesised.saturating_add(1);
                } else {
                    page.file_stated = page.file_stated.saturating_add(1);
                }
                let slot = page.modes.entry(format!("{blend:?}")).or_default();
                *slot = slot.saturating_add(1);
            }
        }
        walk(commands, page);
    }
}

/// Every command list one display list holds.
///
/// Three, not one: §11.4.7's page pair is two lists of the same page, and §11.6.5.1's soft
/// masks are groups whose elements are stored beside the commands rather than among them. A
/// group can stand in any of the three.
fn walk_all(list: &DisplayList, page: &mut Page) {
    walk(list.commands(), page);
    for index in 0..list.soft_mask_count() {
        if let Some(mask) = list.soft_mask(pdf_render::SoftMaskId::new(
            u32::try_from(index).unwrap_or(u32::MAX),
        )) {
            walk(&mask.commands, page);
        }
    }
    if let Some(black) = list.black() {
        walk_all(black, page);
    }
}

/// Interprets one document's pages.
fn examine(path: &str, max_pages: usize) -> Says {
    let mut says = Says::default();
    let Ok(bytes) = std::fs::read(path) else {
        return says;
    };
    let Ok(document) = Document::open(bytes) else {
        return says;
    };
    says.opened = 1;
    if !states_a_blend_mode(&document) {
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
    for index in 0..pages.len().min(max_pages) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        says.pages = says.pages.saturating_add(1);
        let interpreted = pdf_model::interpret(&document, &page);
        let mut found = Page::default();
        walk_all(&interpreted.display_list, &mut found);
        says.groups = says.groups.saturating_add(found.groups);
        if !found.touched() {
            continue;
        }
        says.non_isolated = says.non_isolated.saturating_add(found.non_isolated);
        says.second_run = says.second_run.saturating_add(found.second_run);
        says.second_run_blending_element = says
            .second_run_blending_element
            .saturating_add(found.second_run_blending_element);
        says.synthesised = says.synthesised.saturating_add(found.synthesised);
        says.file_stated = says.file_stated.saturating_add(found.file_stated);
        says.pages_second_run = says
            .pages_second_run
            .saturating_add(usize::from(found.second_run > 0));
        says.pages_synthesised = says
            .pages_synthesised
            .saturating_add(usize::from(found.synthesised > 0));
        says.pages_file_stated = says
            .pages_file_stated
            .saturating_add(usize::from(found.file_stated > 0));
        says.non_isolated_knockout = says
            .non_isolated_knockout
            .saturating_add(found.non_isolated_knockout);
        says.non_isolated_normal = says
            .non_isolated_normal
            .saturating_add(found.non_isolated_normal);
        says.non_isolated_with_a_space = says
            .non_isolated_with_a_space
            .saturating_add(found.non_isolated_with_a_space);
        for (mode, count) in found.modes {
            let slot = says.modes.entry(mode).or_default();
            *slot = slot.saturating_add(count);
        }
        if found.second_run > 0 {
            reached.push(format!(
                "page {} ({} second-run group(s): {} built by \u{a7}11.7.4, {} stated by the \
                 file; {} holding an element that blends)",
                index.saturating_add(1),
                found.second_run,
                found.synthesised,
                found.file_stated,
                found.second_run_blending_element,
            ));
        }
    }
    if !reached.is_empty() {
        says.documents_second_run = 1;
        says.witnesses
            .push(format!("{path} \u{2014} {name}: {}", reached.join("; ")));
    }
    says
}

/// How many pages of each document to walk, and the paths to walk.
///
/// The arguments, the lines of any argument beginning with `@`, and `--pages N` for a walk
/// deeper than the first page.
fn arguments() -> (usize, Vec<String>) {
    let mut max_pages = 1;
    let mut out = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--pages" {
            match args.next().as_deref().map(str::parse::<usize>) {
                Some(Ok(pages)) if pages > 0 => max_pages = pages,
                _ => println!("--pages wants a positive number"),
            }
            continue;
        }
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => println!("{list}: {error}"),
            },
            None => out.push(argument),
        }
    }
    (max_pages, out)
}

fn main() {
    let (max_pages, paths) = arguments();
    eprintln!(
        "{} PDF(s) in the population, {max_pages} page(s) each",
        paths.len()
    );

    let says = paths
        .par_iter()
        .map(|path| examine(path, max_pages))
        .reduce(Says::default, |mut total, one| {
            total.absorb(one);
            total
        });

    // What it matched, before the count that summarises it.
    for witness in &says.witnesses {
        println!("{witness}");
    }

    println!(
        "\n{} path(s), {} opened, {} state no /BM anywhere, {} interpreted over {} page(s) \
         holding {} group(s)",
        paths.len(),
        says.opened,
        says.without_the_parameter,
        says.interpreted,
        says.pages,
        says.groups
    );
    println!("  {} non-isolated group(s) in all", says.non_isolated);
    println!(
        "  {} of them state \u{a7}11.4.4's result step ({} page(s), {} document(s)): {} built by \
         \u{a7}11.7.4's implicit constructions, {} stated by the file",
        says.second_run,
        says.pages_second_run,
        says.documents_second_run,
        says.synthesised,
        says.file_stated
    );
    println!(
        "      {} page(s) hold one \u{a7}11.7.4 built, {} page(s) hold one the file stated",
        says.pages_synthesised, says.pages_file_stated
    );
    println!(
        "      {} of them hold an element that blends, which is where NOTE 3's removal changes \
         a pixel rather than costing one",
        says.second_run_blending_element
    );
    for (mode, count) in &says.modes {
        println!("      {count:>6}  under {mode}");
    }
    println!(
        "  excluded by the predicate: {} non-isolated knockout group(s) (ADR 0327's \
         construction), {} under Normal (NOTE 3's collapse)",
        says.non_isolated_knockout, says.non_isolated_normal
    );
    println!(
        "  {} non-isolated group(s) carrying a blending colour space, which is this census's \
         own check and not a finding",
        says.non_isolated_with_a_space
    );
}
