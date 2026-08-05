//! Which of Table 100's usage categories the corpus actually asks for.
//!
//! §8.11.4.4's usage application dictionaries are applied for the `View` event, and three of the
//! categories are questions about the *processor* rather than about the document: `Zoom`, `User`
//! and `Language`. `optional_content.rs` answers `Zoom` at a magnification of 1.0 and reports the
//! other two rather than guessing, and both of those are **choices** — so the number that decides
//! how much they cost is how many documents state them, which nothing measured until the
//! three-hundred-and-twenty-fourth session.
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

        // Every configuration, not only `/D`: §8.11.4.3's `/Configs` array holds alternatives a
        // processor may offer, and each may state its own `/AS`.
        let mut configurations = vec![document.get_key(properties, "D")];
        if let Some(list) = document.get_key(properties, "Configs").as_array() {
            configurations.extend(list.iter().map(|entry| document.resolve(entry)));
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
                let stated_groups = document.get_key(application, "OCGs");
                if let Some(groups) = stated_groups.as_array() {
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
            }
        }
        if stated {
            with_applications = with_applications.saturating_add(1);
            named.push(path.rsplit('/').next().unwrap_or(&path).to_owned());
        }
    }

    println!("{documents} document(s) opened, {with_properties} with /OCProperties");
    println!("{with_applications} with at least one §8.11.4.4 usage application dictionary");
    println!("  events:     {events:?}");
    println!("  categories: {categories:?}");
    println!("  /Usage entries on the groups they name: {usage_entries:?}");
    for name in &named {
        println!("  {name}");
    }
}
