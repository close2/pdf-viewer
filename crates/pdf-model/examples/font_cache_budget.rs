//! What each font-cache budget gives up, over a run of pages.
//!
//! `pdf_model::FONT_BUDGET`'s *floor* is derived from this, in the form
//! `pdf_syntax::DECODED_BUDGET`'s is: a cache is only worth the bound it is held under if a
//! smaller bound would cost something, so the question is answered by running the real cache at
//! several budgets over the real page sequence rather than by simulating one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example font_cache_budget -- 100
//! cargo run --release -p pdf-model --example font_cache_budget -- \
//!     10 doc/pdf.js/test/pdfs/issue6127.pdf
//! ```
//!
//! A budget of 0 is the cache off — nothing fits under it — and is the row every other row is
//! read against. `misses` is what a load costs somebody: it is the number of times a lookup was
//! not answered, and it is roughly twice the number of loads: `Tf`'s route asks in
//! `Interpreter::font` and again in `Interpreter::load_font` — see the comment on the first of
//! those for why — while Table 57's `/Font` reaches only the second.
//!
//! A third argument runs **one** budget and prints the peak resident memory beside it, which is
//! the number the *ceiling* is argued from: what a cache charges itself is font-program bytes,
//! and what it costs the process is those plus every table a loaded font builds beside them.
//! One budget per process, because a high-water mark does not come back down.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

/// The process's high-water resident memory, in kilobytes, out of `/proc/self/status`.
fn peak_resident() -> String {
    std::fs::read_to_string("/proc/self/status").map_or_else(
        |_| "?".to_owned(),
        |status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("VmHWM:"))
                .map_or_else(|| "?".to_owned(), |value| value.trim().to_owned())
        },
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count = args.next().map_or(100, |n| n.parse().unwrap_or(100));
    let path = args.next().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
            .to_string_lossy()
            .into_owned()
    });

    let Ok(bytes) = std::fs::read(&path) else {
        println!("{path}: unreadable");
        return;
    };
    let Ok(document) = pdf_syntax::Document::open(bytes) else {
        println!("{path}: unopened");
        return;
    };
    let state = pdf_model::view::ViewState::of(&document);
    let pages = pdf_model::Pages::new(&document);

    let only: Option<usize> = args.next().and_then(|n| n.parse().ok());
    println!("{path}, {count} pages");
    println!("budget\tfonts\tbytes\thits\tmisses\tevicted\tpeak-kB");
    let budgets: Vec<usize> = only.map_or_else(
        || {
            vec![
                0,
                64 * 1024,
                256 * 1024,
                512 * 1024,
                1024 * 1024,
                2 * 1024 * 1024,
                4 * 1024 * 1024,
                8 * 1024 * 1024,
            ]
        },
        |budget| vec![budget],
    );
    for budget in budgets {
        let cache = pdf_model::FontCache::with_budget(budget);
        for index in 0..count {
            let Some(page) = pages.get(index) else { break };
            drop(pdf_model::content::interpret_with_fonts(
                &document, &page, &state, &cache,
            ));
        }
        let report = cache.report();
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            report.budget,
            report.fonts,
            report.bytes,
            report.hits,
            report.misses,
            report.evicted,
            peak_resident()
        );
    }
}
