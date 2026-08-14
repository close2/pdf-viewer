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
///
/// **The type is a filter and not a label, since the five-hundred-and-eleventh session.** It read
/// as prose beside the count until then, and one row is two flags: bit 26 is `RadiosInUnison` on a
/// `Btn` and `RichText` on a `Tx`, and a census that counted them together could not answer the
/// question `doc/todo/30` asks — does any document set `RadiosInUnison`? Table 226's `/FT` is
/// inheritable, so it is walked exactly as `/Ff` is.
struct Flag {
    bit: u32,
    name: &'static str,
    kind: &'static str,
}

impl Flag {
    /// Whether this flag applies to a field of this type.
    ///
    /// A field stating no `/FT` anywhere in its ancestry matches nothing type-specific: Table 226
    /// makes the entry "(Required for terminal fields; inheritable)", and a flag word on a field
    /// with no type says nothing about which table's meaning its bits carry.
    fn applies_to(&self, field_type: Option<&str>) -> bool {
        if self.kind.is_empty() {
            return true;
        }
        field_type.is_some_and(|stated| self.kind.split('/').any(|wanted| wanted == stated))
    }
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
        name: "RadiosInUnison",
        kind: "Btn",
    },
    Flag {
        bit: 26,
        name: "RichText",
        kind: "Tx",
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
    // §12.7.5.2.4's population, which no *flag* count can find: a radio field two of whose widgets
    // answer to the same `/AP /N` on-state name. With bit 26 set the clause says all of them go on
    // together; with it clear it says "at most one radio button in a field shall be set at a
    // time", and one `/V` cannot name which. Counted separately because only the second is a
    // decision this reader has to make.
    let mut sharing_with_the_flag: Vec<String> = Vec::new();
    let mut sharing_without_the_flag: Vec<String> = Vec::new();

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
        for (field, identifiers) in &table {
            let mut states: Vec<String> = Vec::new();
            let mut radio = false;
            let mut in_unison = false;
            for identifier in identifiers {
                let object = document.get(*identifier);
                let Some(widget) = object.as_dict() else {
                    continue;
                };
                widgets = widgets.saturating_add(1);
                let flags = inherited_flags(&document, widget);
                let field_type = inherited_type(&document, widget);
                for flag in FLAGS {
                    if flags & (1_i64 << (flag.bit.saturating_sub(1))) == 0
                        || !flag.applies_to(field_type.as_deref())
                    {
                        continue;
                    }
                    let counter = set.entry(flag.name).or_default();
                    *counter = counter.saturating_add(1);
                    let names = documents_setting.entry(flag.name).or_default();
                    if names.last() != Some(&name) {
                        names.push(name.clone());
                    }
                }
                if field_type.as_deref() == Some("Btn") && flags & (1 << 15) != 0 {
                    radio = true;
                    in_unison = flags & (1 << 25) != 0;
                    states.extend(on_states(&document, widget));
                }
            }
            if radio {
                let mut seen = std::collections::BTreeSet::new();
                if states.iter().any(|state| !seen.insert(state.clone())) {
                    let row = format!("{name} {field}");
                    if in_unison {
                        sharing_with_the_flag.push(row);
                    } else {
                        sharing_without_the_flag.push(row);
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

    println!(
        "\n§12.7.5.2.4's own population — a radio field whose widgets share an /AP /N on state:\n  \
         with RadiosInUnison set:   {:>3} field(s){}\n  \
         with it clear:             {:>3} field(s){}",
        sharing_with_the_flag.len(),
        witnesses(&sharing_with_the_flag),
        sharing_without_the_flag.len(),
        witnesses(&sharing_without_the_flag),
    );
}

/// The names of `/AP /N`'s entries that are not the off state, for one widget.
///
/// §12.7.5.2.3 names the off state — "[t]he appearance for the off state is optional but, if
/// present, shall be stored in the appearance dictionary under the name Off" — so every other key
/// of that dictionary is a state that turns the widget on.
fn on_states(document: &Document, widget: &Dictionary) -> Vec<String> {
    let appearances = document.get_key(widget, "AP");
    let Some(appearances) = appearances.as_dict() else {
        return Vec::new();
    };
    let normal = document.get_key(appearances, "N");
    let Some(states) = normal.as_dict() else {
        return Vec::new();
    };
    states
        .iter()
        .map(|(name, _)| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .filter(|name| name != "Off")
        .collect()
}

/// The witnesses, where there are few enough to read.
fn witnesses(found: &[String]) -> String {
    if found.is_empty() || found.len() > 12 {
        return String::new();
    }
    format!(": {}", found.join(", "))
}

/// Table 226's `/FT`, taken from the nearest ancestor that states one (§12.7.4.1).
fn inherited_type(document: &Document, widget: &Dictionary) -> Option<String> {
    let mut current = widget.clone();
    for _ in 0..MAX_ANCESTRY {
        if let Some(name) = document.get_key(&current, "FT").as_name() {
            return Some(String::from_utf8_lossy(name.as_bytes()).into_owned());
        }
        let parent = document.get_key(&current, "Parent");
        let parent = parent.as_dict()?;
        current = parent.clone();
    }
    None
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
