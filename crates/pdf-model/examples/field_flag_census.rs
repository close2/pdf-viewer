//! Which of §12.7's field flags the corpus actually states.
//!
//! Table 227 gives every field three flags, and Tables 229, 231 and 233 give each field type its
//! own — twenty in all, and most of them bind a program that *fills* a field rather than one that
//! draws it. This tree became such a program in the hundred-and-thirty-fifth session, so the
//! question "which of them does any real document set" stopped being idle: a flag no file states
//! is a clause with no witness, and one with a witness is work.
//!
//! ```sh
//! cargo run --release -p pdf-model --example field_flag_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document};

/// One flag: the bit number Table 227, 229, 231 or 233 gives it, its name, and the field types
/// it applies to (`""` for all of them).
struct Flag {
    bit: u32,
    name: &'static str,
    kind: &'static str,
}

/// The twenty flags §12.7.5's four tables state, in bit order.
const FLAGS: &[Flag] = &[
    Flag {
        bit: 1,
        name: "ReadOnly",
        kind: "",
    },
    Flag {
        bit: 2,
        name: "Required",
        kind: "",
    },
    Flag {
        bit: 3,
        name: "NoExport",
        kind: "",
    },
    Flag {
        bit: 13,
        name: "Multiline",
        kind: "Tx",
    },
    Flag {
        bit: 14,
        name: "Password",
        kind: "Tx",
    },
    Flag {
        bit: 15,
        name: "NoToggleToOff",
        kind: "Btn",
    },
    Flag {
        bit: 16,
        name: "Radio",
        kind: "Btn",
    },
    Flag {
        bit: 17,
        name: "Pushbutton",
        kind: "Btn",
    },
    Flag {
        bit: 18,
        name: "Combo",
        kind: "Ch",
    },
    Flag {
        bit: 19,
        name: "Edit",
        kind: "Ch",
    },
    Flag {
        bit: 20,
        name: "Sort",
        kind: "Ch",
    },
    Flag {
        bit: 21,
        name: "FileSelect",
        kind: "Tx",
    },
    Flag {
        bit: 22,
        name: "MultiSelect",
        kind: "Ch",
    },
    Flag {
        bit: 23,
        name: "DoNotSpellCheck",
        kind: "Tx/Ch",
    },
    Flag {
        bit: 24,
        name: "DoNotScroll",
        kind: "Tx",
    },
    Flag {
        bit: 25,
        name: "Comb",
        kind: "Tx",
    },
    Flag {
        bit: 26,
        name: "RadiosInUnison/RichText",
        kind: "Btn/Tx",
    },
    Flag {
        bit: 27,
        name: "CommitOnSelChange",
        kind: "Ch",
    },
];

/// How far §12.7.4.1's `/Parent` chain is followed, matching `appearance.rs`'s own bound.
const MAX_ANCESTRY: usize = 32;

fn main() {
    let mut documents = 0_usize;
    let mut with_form = 0_usize;
    let mut widgets = 0_usize;
    let mut set: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut documents_setting: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let table = pdf_model::view::widgets_by_field_name(&document);
        if table.is_empty() {
            continue;
        }
        with_form = with_form.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        for identifiers in table.values() {
            for identifier in identifiers {
                let object = document.get(*identifier);
                let Some(widget) = object.as_dict() else {
                    continue;
                };
                widgets = widgets.saturating_add(1);
                let flags = inherited_flags(&document, widget);
                for flag in FLAGS {
                    if flags & (1_i64 << (flag.bit.saturating_sub(1))) == 0 {
                        continue;
                    }
                    let counter = set.entry(flag.name).or_default();
                    *counter = counter.saturating_add(1);
                    let names = documents_setting.entry(flag.name).or_default();
                    if names.last() != Some(&name) {
                        names.push(name.clone());
                    }
                }
            }
        }
    }

    println!("{documents} document(s) opened, {with_form} with an /AcroForm, {widgets} widget(s)");
    for flag in FLAGS {
        let count = set.get(flag.name).copied().unwrap_or_default();
        let files = documents_setting
            .get(flag.name)
            .map(Vec::len)
            .unwrap_or_default();
        println!(
            "  bit {:>2} {:<24} {:<6} {count:>5} widget(s) over {files:>3} document(s)",
            flag.bit, flag.name, flag.kind
        );
    }
    for flag in FLAGS {
        let Some(names) = documents_setting.get(flag.name) else {
            continue;
        };
        if names.len() > 12 {
            continue;
        }
        println!("  {}: {}", flag.name, names.join(" "));
    }
}

/// Table 227's `/Ff`, taken from the nearest ancestor that states one (§12.7.4.1).
fn inherited_flags(document: &Document, widget: &Dictionary) -> i64 {
    let mut current = widget.clone();
    for _ in 0..MAX_ANCESTRY {
        if let Some(flags) = document.get_key(&current, "Ff").as_integer() {
            return flags;
        }
        let parent = document.get_key(&current, "Parent");
        let Some(parent) = parent.as_dict() else {
            return 0;
        };
        current = parent.clone();
    }
    0
}
