//! Fuzzes the activation of a hybrid form's `FormCalc` calculations: the order of XFA 3.3 page
//! 379, the cascade by dependency of pages 379–380 and the limits that end it (ADR 2506).
//!
//! **Every script is the producer's.** The input is a stimulus byte and then up to eight scripts
//! separated by NUL bytes, read lossily; script `n` is the calculation of the field `F<n>`, so the
//! fields read and assign one another by name and the template makes whatever cycles, chains and
//! fan-outs the scripts spell. The runner is the one the tests and the census use, included from
//! `crates/pdf-formcalc/tests/common/hosted.rs`, so that what is fuzzed is what ships.
//!
//! Four properties beyond not panicking:
//!
//! - **It ends.** The limits are small here ([`LIMITS`]) so that they are reached often, and an
//!   activation takes less than [`ESCAPED`] of wall time, so that a ceiling it did not enforce is
//!   a failure and not libFuzzer's timeout.
//! - **A limit drops the edits.** An activation that ended on a limit reports no edit.
//! - **The work is counted.** No more scripts ran than the limit allows, and the report holds the
//!   sentences of its cap, the limit's own and one that says what was left out.
//! - **An activation is a function of its input.** Two activations of the same form agree.

#![no_main]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use libfuzzer_sys::fuzz_target;
use pdf_model::xfa::{Activation, Change, Ended, Limits, Resource, Stimulus};
use pdf_syntax::Document;

#[path = "../../crates/pdf-formcalc/tests/common/hosted.rs"]
#[expect(
    dead_code,
    reason = "the file also holds the per-script adapter its other includers use, and this target runs activations"
)]
mod hosted;

use hosted::FormCalcRunner;

/// The most one activation may take: far above what [`LIMITS`]' steps cost even on a machine busy
/// with builds, so that only a ceiling the activation failed to enforce reaches it.
const ESCAPED: Duration = Duration::from_secs(5);

/// Small limits, so that a campaign runs many inputs and ends on each of them often.
const LIMITS: Limits = Limits {
    activations: 200,
    steps: 100_000,
    names: 10_000,
    bytes: 1 << 20,
};

/// The most sentences an activation reports before it says how many it left out.
const REPORT_CAP: usize = 256;

/// How many fields the form holds.
const FIELDS: usize = 8;

/// A script as XML text: markup characters escaped and the control characters XML 1.0 forbids
/// left out.
fn escaped(script: &str) -> String {
    let mut out = String::with_capacity(script.len());
    for c in script.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if c.is_control() && !matches!(c, '\t' | '\n') => {}
            c => out.push(c),
        }
    }
    out
}

/// A stream object holding `text`.
fn stream(text: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{text}\nendstream",
        text.len().saturating_add(1)
    )
}

/// A hybrid form of `scripts.len()` text fields `F0`, `F1`, … each valued 1, `F<n>` calculated by
/// `scripts[n]`, with a widget for each.
fn document(scripts: &[&str]) -> Option<Document> {
    let mut fields = String::new();
    let mut data = String::new();
    for (n, script) in scripts.iter().enumerate() {
        let _ = write!(
            fields,
            "<field name=\"F{n}\"><ui><textEdit/></ui><value><text/></value>\
             <calculate><script>{}</script></calculate></field>",
            escaped(script)
        );
        let _ = write!(data, "<F{n}>1</F{n}>");
    }
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\">{fields}</subform></template>"
    );
    let datasets = format!(
        "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1>\
         {data}</form1></xfa:data></xfa:datasets>"
    );
    // The datasets stream is object 6 and the widgets follow it.
    let widgets: Vec<String> = (0..scripts.len())
        .map(|n| format!("{} 0 R", n.saturating_add(7)))
        .collect();
    let widgets = widgets.join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{widgets}] >>"),
        format!("<< /T (form1[0]) /Kids [{widgets}] >>"),
        stream(&template),
        stream(&datasets),
    ];
    for n in 0..scripts.len() {
        objects.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T (F{n}[0]) /P 3 0 R \
             /V (1) /Rect [72 700 300 720] /DA (/Helv 10 Tf 0 g) >>"
        ));
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).ok()
}

/// One activation of the form `resource` reads, over the fields' values.
fn activate(
    resource: &Resource,
    fields: &[(String, Change)],
    stimulus: Stimulus<'_>,
) -> Activation {
    resource.activate(fields, stimulus, &mut FormCalcRunner, LIMITS)
}

fuzz_target!(|data: &[u8]| {
    let Some((&mode, rest)) = data.split_first() else {
        return;
    };
    let text = String::from_utf8_lossy(rest);
    let scripts: Vec<&str> = text.split('\0').take(FIELDS).collect();
    let Some(document) = document(&scripts) else {
        return;
    };
    let Some(Ok(resource)) = Resource::read(&document) else {
        return;
    };
    let fields: Vec<(String, Change)> = (0..scripts.len())
        .map(|n| (format!("form1[0].F{n}[0]"), Change::Text("1".to_owned())))
        .collect();
    let typed = vec!["form1[0].F0[0]".to_owned()];
    let stimulus = if mode & 1 == 0 {
        Stimulus::Merge
    } else {
        Stimulus::Entry(&typed)
    };
    let started = Instant::now();
    let first = activate(&resource, &fields, stimulus);
    assert!(
        started.elapsed() <= ESCAPED,
        "an activation took {:?}, past the limits' reach",
        started.elapsed()
    );
    assert!(
        !matches!(first.ended, Ended::Budget(_)) || first.edits.is_empty(),
        "an activation that ended on a limit reported edits"
    );
    assert!(
        first.activations <= LIMITS.activations,
        "{} scripts ran past the limit of {}",
        first.activations,
        LIMITS.activations
    );
    assert!(
        first.report.len() <= REPORT_CAP.saturating_add(2),
        "a report of {} sentences passed its cap",
        first.report.len()
    );
    assert!(
        first == activate(&resource, &fields, stimulus),
        "the same form gave two activations"
    );
});
