//! Table 147 print preferences at the actual GTK settings boundary.

use std::fmt::Write as _;

use pdf_model::viewer_preferences::{Duplex, PrintScaling, ViewerPreferences};
use pdf_syntax::Document;
use viewer_host::PrintDefaults;

fn document(duplex: &str) -> Document {
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << \
         /Duplex {duplex} /PrintScaling /None /PrintPageRange [2 2 3 3] \
         /NumCopies 3 >> >>"
    );
    let objects = [
        catalog.as_str(),
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("exact-xref print-preference PDF");
    assert!(!document.was_recovered());
    document
}

fn settings(duplex: &str) -> gtk4::PrintSettings {
    let document = document(duplex);
    let preferences = ViewerPreferences::read(&document);
    assert_eq!(preferences.print_scaling, PrintScaling::NoScaling);
    assert_eq!(preferences.print_page_range, vec![(2, 2), (3, 3)]);
    assert_eq!(preferences.num_copies, Some(3));
    let defaults = PrintDefaults::of(&preferences);
    super::print_settings(&defaults)
}

#[expect(
    clippy::float_cmp,
    reason = "100 is an exact stored setting, not a computed approximation"
)]
#[gtk4::test]
fn pdf_preferences_reach_gtk_settings_with_independent_range_and_duplex_values() {
    use gtk4::{PrintDuplex, PrintPages};

    // GTK's `Vertical` is duplex tumble (short edge) and `Horizontal` is
    // duplex no-tumble (long edge); the GTK 4.18.6 CUPS backend maps these values.
    // https://raw.githubusercontent.com/GNOME/gtk/4.18.6/modules/printbackends/gtkprintbackendcups.c
    for (pdf_name, reader_value, gtk_value) in [
        (
            "/DuplexFlipShortEdge",
            Duplex::FlipShortEdge,
            PrintDuplex::Vertical,
        ),
        (
            "/DuplexFlipLongEdge",
            Duplex::FlipLongEdge,
            PrintDuplex::Horizontal,
        ),
    ] {
        let settings = settings(pdf_name);
        assert_eq!(settings.duplex(), gtk_value, "PDF /Duplex {pdf_name}");
        assert_eq!(settings.n_copies(), 3, "PDF /NumCopies 3");
        assert_eq!(
            settings.scale(),
            100.0,
            "PDF /PrintScaling /None is 100% scale"
        );
        assert_eq!(settings.print_pages(), PrintPages::Ranges);
        assert_eq!(
            settings
                .page_ranges()
                .iter()
                .map(|range| (range.start(), range.end()))
                .collect::<Vec<_>>(),
            vec![(1, 1), (2, 2)],
            "Table 147 pairs [2 2] and [3 3] are inclusive and one-based; GtkPageRange is zero-based"
        );
        let preferences = ViewerPreferences::read(&document(pdf_name));
        assert_eq!(preferences.duplex, Some(reader_value));
    }
}
