//! Which of §12.7's field flags the corpus actually states.
//!
//! Table 227 gives every field three flags, and Tables 229, 231 and 233 give each field type its
//! own — twenty in all, and most of them bind a program that *fills* a field rather than one that
//! draws it. This tree became such a program in the hundred-and-thirty-fifth session, so the
//! question "which of them does any real document set" stopped being idle: a flag no file states
//! is a clause with no witness, and one with a witness is work.
//!
//! It counts two things beside the flags, because both are read off the same walk of the field
//! tree and neither is a flag. Table 231 bit 25's own condition — "[m]ay be set only if the MaxLen
//! entry is present in the text field dictionary" — is what a comb layout needs beside the flag,
//! and Table 192's `/R` turns a widget's whole appearance in quarter steps. The `/DA` text matrix
//! that turns the text *inside* a widget is a different rotation over a wider population, and
//! `examples/variable_text_census` counts that one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example field_flag_census -- doc/pdf.js/test/pdfs/*.pdf
//! cargo run --release -p pdf-model --example field_flag_census -- @<list-of-paths>
//! ```
//!
//! The second form is the crawl's: 65 944 paths are more than one command line holds, and a
//! census split into chunks prints a dozen partial answers instead of one number.

#![expect(
    clippy::doc_markdown,
    reason = "the module doc quotes Table 231 verbatim, and a quotation is not marked up"
)]
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

#[expect(
    clippy::too_many_lines,
    reason = "a census main is one straight sweep — open, walk the field tree, tally the three \
bit-26 populations, print — and splitting it would scatter the tally the report reads back"
)]
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
    // §12.7.5.3 bit 26's own population, which the flag count above cannot answer either. The
    // bit says "the value of this field shall be a rich text string" and points at Table 228's
    // `/RV` for it; §12.7.4.3 then writes what a *processor* does — "[f]or these fields, the
    // following conventions are not used, and the entire annotation appearance shall be
    // regenerated each time the value is changed". So what costs a page is not the flag but the
    // three together: the flag, an `/RV` there is formatting in, and a document whose
    // `/NeedAppearances` or whose reader's edit reaches the regeneration at all.
    let mut rich_text = 0_usize;
    let mut rich_text_with_rv = 0_usize;
    let mut rich_text_regenerated: Vec<String> = Vec::new();
    // Table 231 bit 25's own population. The flag is counted above; what a layout needs beside it
    // is `/MaxLen`, which the bit's own condition requires ("[m]ay be set only if the MaxLen entry
    // is present"), and whether the `/DA` states a `Tm` for the cells to be written under.
    let mut comb_with_max_len = 0_usize;
    let mut comb_without_max_len: Vec<String> = Vec::new();
    // Table 192's `/R`, the other rotation — of the widget's whole appearance rather than of the
    // text inside it, and in quarter steps rather than by an arbitrary matrix.
    let mut widget_rotation: BTreeMap<i64, usize> = BTreeMap::new();

    for path in paths() {
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
        let need_appearances = need_appearances(&document);
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
                // Every field type whose widget §12.7.4.3 lays text into: a text field's value,
                // a choice field's options and a button's caption all go through one layout, and
                // Table 192's `/R` turns all three.
                if matches!(field_type.as_deref(), Some("Tx" | "Ch" | "Btn")) {
                    if field_type.as_deref() == Some("Tx") && flags & (1 << 24) != 0 {
                        if inherited(&document, widget, "MaxLen").is_some() {
                            comb_with_max_len = comb_with_max_len.saturating_add(1);
                        } else {
                            comb_without_max_len.push(format!("{name} {field}"));
                        }
                    }
                    let turn = document
                        .get_key(widget, "MK")
                        .as_dict()
                        .map(|characteristics| document.get_key(characteristics, "R"))
                        .and_then(|value| value.as_integer())
                        .unwrap_or_default();
                    let counter = widget_rotation.entry(turn).or_default();
                    *counter = counter.saturating_add(1);
                }
                if field_type.as_deref() == Some("Tx") && flags & (1 << 25) != 0 {
                    rich_text = rich_text.saturating_add(1);
                    if inherited(&document, widget, "RV").is_some() {
                        rich_text_with_rv = rich_text_with_rv.saturating_add(1);
                        if need_appearances {
                            rich_text_regenerated.push(format!("{name} {field}"));
                        }
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
        "\n§12.7.5.3 bit 26's own population — a /Tx field whose value the clause makes rich \
         text:\n  \
         RichText set:              {rich_text:>3} widget(s)\n  \
         …and stating Table 228's /RV: {rich_text_with_rv:>3} widget(s)\n  \
         …in a /NeedAppearances document: {:>3} widget(s){}",
        rich_text_regenerated.len(),
        witnesses(&rich_text_regenerated),
    );

    println!(
        "\nTable 231 bit 25's own population — a comb field and whether its own condition \
         holds:\n  \
         Comb with Table 232's /MaxLen:  {comb_with_max_len:>3} widget(s)\n  \
         Comb without one:               {:>3} widget(s){}",
        comb_without_max_len.len(),
        witnesses(&comb_without_max_len),
    );

    println!("\nTable 192's /R, the widget appearance's own quarter turn:");
    for (degrees, count) in &widget_rotation {
        println!("  {degrees:>4} degrees {count:>5} widget(s)");
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

/// Table 224's `/NeedAppearances`, "a flag specifying whether to construct appearance streams
/// and appearance dictionaries for all widget annotations in the document".
fn need_appearances(document: &Document) -> bool {
    let Ok(catalog) = document.catalog() else {
        return false;
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return false;
    };
    matches!(
        document.get_key(form, "NeedAppearances"),
        pdf_syntax::Object::Boolean(true)
    )
}

/// One entry taken from the nearest ancestor that states it (§12.7.4.1), for the entries this
/// census reads that are neither `/FT` nor `/Ff`.
fn inherited(document: &Document, widget: &Dictionary, key: &str) -> Option<pdf_syntax::Object> {
    let mut current = widget.clone();
    for _ in 0..MAX_ANCESTRY {
        let value = document.get_key(&current, key);
        if !matches!(value, pdf_syntax::Object::Null) {
            return Some(value);
        }
        let parent = document.get_key(&current, "Parent");
        let parent = parent.as_dict()?;
        current = parent.clone();
    }
    None
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
