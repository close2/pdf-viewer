//! Which of Table 100's usage categories the corpus actually asks for.
//!
//! §8.11.4.4's usage application dictionaries are applied for the `View` event, and three of the
//! categories are questions about the *processor* rather than about the document: `Zoom`, `User`
//! and `Language`. `optional_content.rs` answers `Zoom` at a magnification of 1.0 and reports the
//! other two rather than guessing, and both of those are **choices** — so the number that decides
//! how much they cost is how many documents state them, which nothing measured until the
//! three-hundred-and-twenty-fourth session.
//!
//! It counts two entries of Table 98 and Table 99 besides, for the same reason and by the same
//! route: `/Configs`, which is what §8.11.4's row names as its own remaining debt, and `/Locked`,
//! which §8.11's row rests a claim on. A grep answers neither — a configuration dictionary is
//! usually inside an object stream — so the population had never been read at all.
//!
//! **And it reads the three categories' *values*, over every group the document declares rather
//! than only over the groups a usage application dictionary names.** Those are two different
//! populations and conflating them would understate the first: Table 100's `/Zoom`, `/User` and
//! `/Language` are entries of a *group's* `/Usage` dictionary, which a document may state with no
//! `/AS` naming the category at all — §8.11.4.4 says the data "serves as information for a
//! document user to examine" whether or not a usage application dictionary consults it. The
//! values decide what a fixture has to look like, which is why they are printed rather than
//! counted.
//!
//! ```sh
//! cargo run --release -p pdf-model --example oc_usage_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::Document;

fn main() {
    let mut documents = 0_usize;
    let mut with_properties = 0_usize;
    let mut with_applications = 0_usize;
    let mut categories: BTreeMap<String, usize> = BTreeMap::new();
    let mut events: BTreeMap<String, usize> = BTreeMap::new();
    let mut usage_entries: BTreeMap<String, usize> = BTreeMap::new();
    let mut named: Vec<String> = Vec::new();
    let mut with_locked = 0_usize;
    let mut locked_named: Vec<String> = Vec::new();
    let mut with_configs = 0_usize;
    let mut declared_entries: BTreeMap<String, usize> = BTreeMap::new();
    let mut values: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let Ok(catalog) = document.catalog() else {
            continue;
        };
        let properties = document.get_key(&catalog, "OCProperties");
        let Some(properties) = properties.as_dict() else {
            continue;
        };
        with_properties = with_properties.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        survey_declared_groups(
            &document,
            properties,
            &name,
            &mut declared_entries,
            &mut values,
        );

        // Every configuration, not only `/D`: §8.11.4.3's `/Configs` array holds alternatives a
        // processor may offer, and each may state its own `/AS`.
        let mut configurations = vec![document.get_key(properties, "D")];
        if let Some(list) = document.get_key(properties, "Configs").as_array() {
            with_configs = with_configs.saturating_add(1);
            configurations.extend(list.iter().map(|entry| document.resolve(entry)));
        }
        if locks_a_group(&document, &configurations) {
            with_locked = with_locked.saturating_add(1);
            locked_named.push(name.clone());
        }
        let mut stated = false;
        for configuration in configurations {
            let Some(configuration) = configuration.as_dict() else {
                continue;
            };
            let stated_applications = document.get_key(configuration, "AS");
            let Some(applications) = stated_applications.as_array() else {
                continue;
            };
            for application in applications {
                let resolved = document.resolve(application);
                let Some(application) = resolved.as_dict() else {
                    continue;
                };
                stated = true;
                if let Some(event) = document.get_key(application, "Event").as_name() {
                    let counter = events
                        .entry(String::from_utf8_lossy(event.as_bytes()).into_owned())
                        .or_default();
                    *counter = counter.saturating_add(1);
                }
                let stated_categories = document.get_key(application, "Category");
                if let Some(list) = stated_categories.as_array() {
                    for category in list {
                        if let Some(name) = document.resolve(category).as_name() {
                            let counter = categories
                                .entry(String::from_utf8_lossy(name.as_bytes()).into_owned())
                                .or_default();
                            *counter = counter.saturating_add(1);
                        }
                    }
                }
                // And what the groups themselves state, which is the other half: a category is
                // answered from the group's own `/Usage` dictionary.
                count_usage_entries(&document, application, &mut usage_entries);
            }
        }
        if stated {
            with_applications = with_applications.saturating_add(1);
            named.push(name);
        }
    }

    report(&Counted {
        documents,
        with_properties,
        with_applications,
        with_configs,
        with_locked,
        locked_named,
        events,
        categories,
        usage_entries,
        declared_entries,
        values,
        named,
    });
}

/// Everything one run counted, so that [`report`] is one function and `main` is one loop.
struct Counted {
    documents: usize,
    with_properties: usize,
    with_applications: usize,
    with_configs: usize,
    with_locked: usize,
    locked_named: Vec<String>,
    events: BTreeMap<String, usize>,
    categories: BTreeMap<String, usize>,
    usage_entries: BTreeMap<String, usize>,
    declared_entries: BTreeMap<String, usize>,
    values: Vec<String>,
    named: Vec<String>,
}

/// Prints one run's counts.
fn report(counted: &Counted) {
    let Counted {
        documents,
        with_properties,
        with_applications,
        with_configs,
        with_locked,
        locked_named,
        events,
        categories,
        usage_entries,
        declared_entries,
        values,
        named,
    } = counted;
    println!("{documents} document(s) opened, {with_properties} with /OCProperties");
    println!("{with_applications} with at least one §8.11.4.4 usage application dictionary");
    println!("{with_configs} with Table 98's /Configs, {with_locked} with Table 99's /Locked");
    for name in locked_named {
        println!("  /Locked: {name}");
    }
    println!("  events:     {events:?}");
    println!("  categories: {categories:?}");
    println!("  /Usage entries on the groups they name: {usage_entries:?}");
    println!("  /Usage entries on every declared group:  {declared_entries:?}");
    for value in values {
        println!("  value: {value}");
    }
    for name in named {
        println!("  {name}");
    }
}

/// Whether any of a document's configurations states Table 99's `/Locked`.
///
/// Counted because it is the entry §8.11's row rests a claim on and no instrument in this tree
/// read it. A grep cannot answer it: a configuration dictionary usually lives in an object stream.
fn locks_a_group(document: &Document, configurations: &[pdf_syntax::Object]) -> bool {
    configurations.iter().any(|configuration| {
        configuration
            .as_dict()
            .is_some_and(|dict| document.get_key(dict, "Locked").as_array().is_some())
    })
}

/// Counts the `/Usage` entries of every group a usage application dictionary names.
fn count_usage_entries(
    document: &Document,
    application: &pdf_syntax::Dictionary,
    usage_entries: &mut BTreeMap<String, usize>,
) {
    let stated_groups = document.get_key(application, "OCGs");
    let Some(groups) = stated_groups.as_array() else {
        return;
    };
    for group in groups {
        let group = document.resolve(group);
        let Some(group) = group.as_dict() else {
            continue;
        };
        let usage = document.get_key(group, "Usage");
        let Some(usage) = usage.as_dict() else {
            continue;
        };
        for (key, _) in usage.iter() {
            let counter = usage_entries
                .entry(String::from_utf8_lossy(key.as_bytes()).into_owned())
                .or_default();
            *counter = counter.saturating_add(1);
        }
    }
}

/// Counts the `/Usage` entries of every group `/OCProperties /OCGs` declares, and prints the
/// values of the three categories that are questions about the processor.
///
/// Table 98 requires `/OCGs` to list every group in the document, so this is the whole
/// population — the one the fixtures have to be built against, since a `/Zoom` range nobody
/// states is a range no test can be calibrated from.
fn survey_declared_groups(
    document: &Document,
    properties: &pdf_syntax::Dictionary,
    file: &str,
    declared_entries: &mut BTreeMap<String, usize>,
    values: &mut Vec<String>,
) {
    let stated = document.get_key(properties, "OCGs");
    let Some(groups) = stated.as_array() else {
        return;
    };
    for group in groups {
        let group = document.resolve(group);
        let Some(group) = group.as_dict() else {
            continue;
        };
        let usage = document.get_key(group, "Usage");
        let Some(usage) = usage.as_dict() else {
            continue;
        };
        for (key, _) in usage.iter() {
            let counter = declared_entries
                .entry(String::from_utf8_lossy(key.as_bytes()).into_owned())
                .or_default();
            *counter = counter.saturating_add(1);
        }
        for category in ["Zoom", "User", "Language"] {
            let stated = document.get_key(usage, category);
            if let Some(dict) = stated.as_dict() {
                let entries: Vec<String> = dict
                    .iter()
                    .map(|(key, _)| {
                        let key = String::from_utf8_lossy(key.as_bytes()).into_owned();
                        let value = document.get_key(dict, &key);
                        format!("{key}={}", legible(&value))
                    })
                    .collect();
                values.push(format!("{file}: /{category} {}", entries.join(" ")));
            }
        }
    }
}

/// One usage entry's value, as a person reads it rather than as a debug format prints it.
fn legible(value: &pdf_syntax::Object) -> String {
    match value {
        pdf_syntax::Object::String(bytes) => format!("({})", pdf_syntax::text_string(bytes)),
        pdf_syntax::Object::Name(name) => {
            format!("/{}", String::from_utf8_lossy(name.as_bytes()))
        }
        other => format!("{other:?}"),
    }
}
