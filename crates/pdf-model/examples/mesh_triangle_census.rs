//! How close a real mesh shading comes to the bound that stops one being read.
//!
//! ISO 32000-2 §8.7.4.5.5 to §8.7.4.5.8's meshes arrive as a compressed bit stream, so a few
//! kilobytes can describe an unbounded number of patches, and `pdf_model::mesh::MAX_TRIANGLES`
//! is where reading one stops. §10.7.3 is the clause that licenses such a bound:
//!
//! > Each output device may have internal limits on the maximum and minimum tolerances
//! > attainable.
//!
//! And the bound is measured in **this program's own triangles** rather than in the
//! document's patches, because a patch becomes `PATCH_STEPS²` quadrilaterals here. So how many
//! patches a document is allowed is a function of a constant that has nothing to do with the
//! document, and the only way to know whether that constant is anywhere near a real file is to
//! count.
//!
//! **The count comes off the display list rather than off the stream**, which is the whole of
//! why this walks pages instead of objects. §8.6.5.1 resolves a `/ColorSpace` stated as a name
//! through "the resource dictionary in force", and the space decides how many components a
//! vertex carries and therefore how the bit stream divides — so a mesh read without the
//! dictionary that names its space is read wrong or not at all, and the dictionary in force is
//! a page's, or a form `XObject`'s inside it, rather than anything the shading object states. A
//! first draft of this census scanned objects and could not build nine of the corpus's
//! twenty-four meshes for exactly that reason, the largest documents among them
//! (`doc/habits.md`, *Measuring*: a measurement is taken with the instrument under test).
//!
//! ```sh
//! cargo run --release -p pdf-model --example mesh_triangle_census -- \
//!     doc/pdf.js/test/pdfs doc/corpora corpus-cache
//! ```
//!
//! With no arguments it walks `doc/pdf.js/test/pdfs`. Every argument is walked recursively.

#![expect(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    reason = "an example whose entire output is a measurement, and whose one ratio is a \
              percentage of counts four orders of magnitude below what an f64 states exactly"
)]

use std::path::{Path, PathBuf};

use rayon::prelude::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_model::{Pages, Unsupported};
use pdf_render::{Command, Paint, ShadingKind};
use pdf_syntax::Document;

/// One mesh command, as the interpreter built it.
struct Mesh {
    /// The document it came out of.
    document: String,
    /// The page it was painted on, one-based.
    page: usize,
    /// Triangles the reader produced.
    triangles: usize,
}

/// What one document answered.
#[derive(Default)]
struct Finding {
    /// Meshes the interpreter painted with.
    meshes: Vec<Mesh>,
    /// Pages whose report names the triangle bound.
    cut: Vec<String>,
}

fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "pdf") {
            found.push(path);
        }
    }
}

fn paths() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        walk(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs"),
            &mut found,
        );
    } else {
        for argument in arguments {
            let path = PathBuf::from(&argument);
            if path.is_dir() {
                walk(&path, &mut found);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every mesh a display list paints with, groups included.
fn meshes_in(commands: &[Command], out: &mut Vec<usize>) {
    for command in commands {
        match command {
            Command::Fill {
                paint: Paint::Shading(shading),
                ..
            } => {
                if let ShadingKind::Mesh { triangles, .. } = shading.kind.as_ref() {
                    out.push(triangles.len());
                }
            }
            Command::Group { commands, .. } => meshes_in(commands, out),
            _ => {}
        }
    }
}

/// Interprets every page of one document and counts the meshes it painted with.
fn examine(path: &Path) -> Finding {
    let mut finding = Finding::default();
    let Ok(bytes) = std::fs::read(path) else {
        return finding;
    };
    let Ok(document) = Document::open(bytes) else {
        return finding;
    };
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let pages = Pages::new(&document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let interpretation = pdf_model::interpret(&document, &page);
        let mut counts = Vec::new();
        meshes_in(interpretation.display_list.commands(), &mut counts);
        if interpretation.unsupported.iter().any(|item| {
            matches!(
                item,
                Unsupported::LimitReached {
                    limit: "max_mesh_triangles"
                }
            )
        }) {
            finding
                .cut
                .push(format!("{name} page {}", index.saturating_add(1)));
        }
        for triangles in counts {
            finding.meshes.push(Mesh {
                document: name.clone(),
                page: index.saturating_add(1),
                triangles,
            });
        }
    }
    finding
}

fn main() {
    let files = paths();
    println!("{} files", files.len());
    let findings: Vec<Finding> = files.par_iter().map(|path| examine(path)).collect();

    let mut meshes: Vec<Mesh> = Vec::new();
    let mut cut: Vec<String> = Vec::new();
    for finding in findings {
        meshes.extend(finding.meshes);
        cut.extend(finding.cut);
    }

    let bound = pdf_model::mesh::MAX_TRIANGLES;
    meshes.sort_by_key(|mesh| std::cmp::Reverse(mesh.triangles));
    println!(
        "{} mesh paints; the bound is {bound} triangles and {} page(s) report it",
        meshes.len(),
        cut.len()
    );
    println!("the twenty largest, as a share of the bound:");
    for mesh in meshes.iter().take(20) {
        println!(
            "  {:>9} {:>6.2}%  {} page {}",
            mesh.triangles,
            mesh.triangles as f64 * 100.0 / bound as f64,
            mesh.document,
            mesh.page
        );
    }
    for page in &cut {
        println!("  cut short: {page}");
    }
}
