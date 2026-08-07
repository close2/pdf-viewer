//! A sidebar's worth of a document, read out of a process that cannot open a file.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example confined_panels -- file.pdf [page]
//! ```
//!
//! The demonstration ADR 0223 owes: the eleven questions that answer with a `pdf-model` type —
//! §12.3.3's outline, §8.11.4.3's layers, §7.11.4's embedded files, §12.3.5's collection,
//! §12.4.3's articles, §12.3.4's thumbnail, §14.3.3's properties, Table 29's opening pair,
//! Table 147, §12.5.6.14's popups and §14.7's structure — asked of a confined viewer and printed.
//! **The document is never parsed in this process.** What this program holds is bytes it read and
//! answers it was handed.
//!
//! Each question is timed, because "what does a panel cost a host on this boundary" is the number
//! this round owed. What each answer costs in *bytes* is the other half and is measured where the
//! encoder is reachable — `protocol`'s `what_each_panel_costs_to_cross`, which prints a table
//! under `--nocapture`.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::panic,
    reason = "an example whose whole output is what it printed; a run that cannot do the thing \
              should stop loudly rather than print a number about something else"
)]

use std::time::Instant;

use viewer_confined::{Confined, Reply};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query};

/// Asks one question, times it, and hands back the answer.
fn ask(confined: &mut Confined, label: &str, query: Query<'_>) -> Reply {
    let at = Instant::now();
    let reply = confined
        .query(query)
        .unwrap_or_else(|error| panic!("{label} crosses: {error}"));
    println!("  {label}: {:.3} ms", at.elapsed().as_secs_f64() * 1e3);
    reply
}

/// Prints an outline level, indented, so that a tree looks like one.
fn print_items(items: &[pdf_model::outline::Item], indent: usize) {
    for item in items {
        println!(
            "    {:indent$}{} {}",
            "",
            if item.open { "v" } else { ">" },
            item.title,
            indent = indent
        );
        print_items(&item.children, indent.saturating_add(2));
    }
}

/// Prints a layer level, likewise.
fn print_layers(layers: &[viewer_core::Layer], indent: usize) {
    for layer in layers {
        match layer {
            viewer_core::Layer::Group {
                name, on, locked, ..
            } => println!(
                "    {:indent$}[{}] {}{}",
                "",
                if *on { "x" } else { " " },
                name.as_deref().unwrap_or("(unnamed group)"),
                if *locked { " (locked)" } else { "" },
                indent = indent
            ),
            viewer_core::Layer::Collection { label, children } => {
                println!(
                    "    {:indent$}{}",
                    "",
                    label.as_deref().unwrap_or("(unlabelled)"),
                    indent = indent
                );
                print_layers(children, indent.saturating_add(2));
            }
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "eleven questions, one after another, and splitting them would put the panel in one \
              function and what it is made of in another"
)]
fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: confined_panels <file.pdf> [page]");
        std::process::exit(2);
    };
    let page: usize = arguments
        .next()
        .map_or(1, |text| text.parse().expect("a page number"));

    let bytes = std::fs::read(&path).expect("the document is readable");
    println!("{path}: {} bytes", bytes.len());

    let started = Instant::now();
    let mut confined = Confined::start().expect("a confined viewer starts");
    let confinement = confined.confinement();
    println!(
        "worker started and confined in {:.3} ms — {}",
        started.elapsed().as_secs_f64() * 1e3,
        confinement.shortfall().unwrap_or_else(|| {
            "seccomp, Landlock and an address-space ceiling, all enforced".to_owned()
        })
    );

    confined
        .handle(&Command::Resize {
            width: 900,
            height: 1200,
            scale: 1.0,
        })
        .expect("a resize crosses");
    let events = confined
        .handle(&Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .expect("an open crosses");
    for event in &events {
        if let Event::Opened { pages, .. } = event {
            println!("{pages} page(s)");
        }
    }
    if page > 1 {
        confined
            .handle(&Command::GoTo(PageTarget::Index(page.saturating_sub(1))))
            .expect("a page turn crosses");
    }

    println!("what the panels are, from inside the confinement:");

    match ask(&mut confined, "outline", Query::Outline) {
        Reply::Outline(outline) => {
            println!(
                "    {} top-level item(s), {} visible, stated count {:?}",
                outline.items.len(),
                outline.visible_count(),
                outline.stated_count
            );
            print_items(&outline.items, 0);
        }
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "layers", Query::Layers) {
        Reply::Layers(layers) => {
            println!("    {} entry(s) in /Order", layers.len());
            print_layers(&layers, 0);
        }
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "attachments", Query::Attachments) {
        Reply::Attachments(attachments) => {
            println!("    {} embedded file(s)", attachments.len());
            for attachment in &attachments {
                println!(
                    "      {} ({:?}, {:?} bytes, {:?})",
                    attachment.name,
                    attachment.media_type.as_deref().unwrap_or("no media type"),
                    attachment.size,
                    attachment.relationship
                );
            }
        }
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "collection", Query::Collection) {
        Reply::Collection(collection) => println!(
            "    {} schema column(s), view {:?}, {} folder(s)",
            collection.schema.len(),
            collection.view,
            collection.all_folders().len()
        ),
        Reply::None => println!("    this document states no portable collection"),
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "articles", Query::Articles) {
        Reply::Articles(threads) => {
            println!("    {} thread(s)", threads.len());
            for thread in &threads {
                println!(
                    "      {} — {} bead(s)",
                    thread.title.as_deref().unwrap_or("(untitled)"),
                    thread.beads.len()
                );
            }
        }
        other => println!("    {other:?}"),
    }

    match ask(
        &mut confined,
        "thumbnail",
        Query::Thumbnail(page.saturating_sub(1)),
    ) {
        Reply::Thumbnail(thumbnail) => println!(
            "    {}x{} samples, {} bytes, colour space permitted {}, subtype permitted {}",
            thumbnail.image.width,
            thumbnail.image.height,
            thumbnail.image.data.len(),
            thumbnail.permitted_colour_space,
            thumbnail.permitted_subtype
        ),
        Reply::None => println!("    this page states no /Thumb"),
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "properties", Query::Properties) {
        Reply::Properties {
            information,
            metadata,
        } => {
            println!(
                "    title {:?}, producer {:?}, trapped {:?}",
                information.title, information.producer, information.trapped
            );
            match metadata {
                None => println!("    no /Metadata"),
                Some(Ok(xmp)) => println!(
                    "    {} XMP property(s), dc:title {:?}",
                    xmp.properties().len(),
                    xmp.title()
                ),
                Some(Err(error)) => println!("    /Metadata refused: {error}"),
            }
        }
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "opening", Query::Opening) {
        Reply::Opening(opening) => println!("    {opening:?}"),
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "preferences", Query::Preferences) {
        Reply::Preferences(preferences) => println!(
            "    display title {}, direction {:?}, print scaling {:?}",
            preferences.display_doc_title, preferences.direction, preferences.print_scaling
        ),
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "popups", Query::Popups) {
        Reply::Popups(popups) => println!("    {} open popup window(s)", popups.len()),
        other => println!("    {other:?}"),
    }

    match ask(&mut confined, "structure", Query::AccessibilityTree) {
        Reply::Accessibility(nodes) => {
            println!("    {} node(s) on this page", nodes.len());
            for node in nodes.iter().take(8) {
                println!(
                    "      {}{} {:?}",
                    if node.parent.is_some() { "  " } else { "" },
                    node.role,
                    node.name.chars().take(48).collect::<String>()
                );
            }
        }
        other => println!("    {other:?}"),
    }
}
