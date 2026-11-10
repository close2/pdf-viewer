//! PDF-input coverage for the remaining Table 57 parameters reviewed in this audit.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "fixtures are valid one-page PDFs, pixel coordinates are in range, and the sampled index is at most 8192"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint, Rasterizer, ShadingKind, TargetSpec};
use pdf_syntax::Document;

/// A small valid PDF whose page resource dictionary contains the supplied entries.
fn fixture(resources: &str, content: &str) -> Vec<u8> {
    fixture_with_page_entries("", resources, content)
}

fn fixture_with_page_entries(page_entries: &str, resources: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] {page_entries} \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn interpretation(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    assert!(
        !document.was_recovered(),
        "the handwritten fixture has a valid cross-reference table and needs no recovery"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the valid fixture should be interpreted completely: {:?}",
        interpretation.unsupported
    );
    interpretation
}

fn assert_function_entries(bytes: Vec<u8>, graphics_state: &str, keys: &[&str]) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    assert!(
        !document.was_recovered(),
        "fixture parsing uses its xref table"
    );
    let catalog = document.catalog().expect("the fixture has a catalog");
    let page_tree = document.get_key(&catalog, "Pages");
    let page_tree = page_tree.as_dict().expect("the catalog has a page tree");
    let kids = document.get_key(page_tree, "Kids");
    let kids = kids.as_array().expect("the page tree has one child");
    let first = kids.first().expect("the fixture has a page");
    let page = document.resolve(first);
    let page = page.as_dict().expect("the page tree child is a page");
    let resources = document.get_key(page, "Resources");
    let resources = resources.as_dict().expect("the page has resources");
    let ext_gstate = document.get_key(resources, "ExtGState");
    let ext_gstate = ext_gstate
        .as_dict()
        .expect("the page has ExtGState resources");
    let state = document.get_key(ext_gstate, graphics_state);
    let state = state
        .as_dict()
        .expect("the named ExtGState is a dictionary");
    for key in keys {
        let function = document.get_key(state, key);
        let function = function
            .as_dict()
            .expect("the named function entry is a dictionary");
        assert_eq!(
            document.get_key(function, "FunctionType").as_integer(),
            Some(2),
            "/{graphics_state}/{key} must be the stated competing function"
        );
    }
}

fn shading_ramp(bytes: Vec<u8>) -> pdf_render::shading::Ramp {
    let result = interpretation(bytes);
    let mut ramps = result.display_list.commands().iter().filter_map(|command| {
        let Command::Fill {
            paint: Paint::Shading(shading),
            ..
        } = command
        else {
            return None;
        };
        match &*shading.kind {
            ShadingKind::Axial { ramp, .. } => Some(ramp.clone()),
            _ => None,
        }
    });
    let ramp = ramps.next().expect("the fixture paints an axial shading");
    assert!(ramps.next().is_none(), "the fixture paints one shading");
    ramp
}

fn centre_pixel(bytes: Vec<u8>) -> [u8; 3] {
    let result = interpretation(bytes);
    let list = &result.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 10 by 10 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((5 * raster.width + 5) * 4) as usize;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// `/SM` in a PDF `ExtGState` improves the sampled ramp's approximation to its function.
#[test]
fn extgstate_smoothness_improves_sampling_accuracy() {
    let shading = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 5 10 5] \
                   /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] \
                   /C0 [0 0 0] /C1 [1 1 1] /N 80 >> >>";
    let resources = |smoothness: &str| {
        format!("/ExtGState << /Fine << {smoothness} >> >> /Shading << /Ramp {shading} >>")
    };
    let fine = shading_ramp(fixture(&resources("/SM 0.0001"), "/Fine gs /Ramp sh"));
    let default = shading_ramp(fixture(&resources(""), "/Fine gs /Ramp sh"));
    let maximum_error = |ramp: &pdf_render::shading::Ramp| {
        (0_u32..=8192)
            .map(|step| {
                let parameter = step as f32 / 8192.0;
                (ramp.colour_at(parameter).r - parameter.powf(80.0)).abs()
            })
            .fold(0.0_f32, f32::max)
    };
    let fine_error = maximum_error(&fine);
    let default_error = maximum_error(&default);
    assert!(
        fine_error < 0.0021,
        "the tight /SM ramp must stay within the device's interpolation bound: {fine_error}"
    );
    assert!(
        default_error > 0.005,
        "the absent-/SM control must show the high-curvature feature lost by default sampling: \
         {default_error}"
    );
}

/// `/TR2 /Default` restores the page-start transfer after an earlier nondefault `/TR`.
#[test]
fn tr2_default_resets_a_transfer_from_an_earlier_gs() {
    let invert = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";
    let resources = format!("/ExtGState << /Set << /TR {invert} >> /Reset << /TR2 /Default >> >>");
    let control = centre_pixel(fixture(&resources, "0.2 0.4 0.6 rg 0 0 10 10 re f"));
    let changed = centre_pixel(fixture(&resources, "/Set gs 0.2 0.4 0.6 rg 0 0 10 10 re f"));
    let reset = centre_pixel(fixture(
        &resources,
        "/Set gs /Reset gs 0.2 0.4 0.6 rg 0 0 10 10 re f",
    ));
    assert_ne!(
        changed, control,
        "the inverting `/TR` control proves the transfer function is exercised"
    );
    assert_eq!(
        reset, control,
        "literal `/TR2 /Default` must clear `/TR` and restore this device's page-start transfer"
    );
}

/// `/TR2` takes precedence over `/TR` when both appear in the same graphics-state dictionary.
#[test]
fn tr2_function_wins_over_tr_in_the_same_gs_dictionary() {
    let invert = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";
    let square = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 2 >>";
    let resources = format!(
        "/ExtGState << /Both << /TR {invert} /TR2 {square} >> \
         /SecondOnly << /TR2 {square} >> /FirstOnly << /TR {invert} >> >>"
    );
    assert_function_entries(
        fixture(&resources, "/Both gs 0.2 0.4 0.6 rg 0 0 10 10 re f"),
        "Both",
        &["TR", "TR2"],
    );
    let page = |setup: &str| {
        centre_pixel(fixture(
            &resources,
            &format!("{setup} 0.2 0.4 0.6 rg 0 0 10 10 re f"),
        ))
    };
    let both = page("/Both gs");
    let second = page("/SecondOnly gs");
    let first = page("/FirstOnly gs");
    assert_eq!(
        both, second,
        "the same-dictionary result is TR2's square map"
    );
    assert_ne!(
        both, first,
        "the competing TR inversion is a negative control"
    );
}

/// `/UCR2` takes precedence over `/UCR` for a `DeviceRGB` mark entering a `DeviceCMYK` group.
#[test]
fn ucr2_function_wins_over_ucr_in_the_same_gs_dictionary() {
    let identity = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>";
    let invert = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";
    let resources = format!(
        "/ExtGState << /Both << /UCR {identity} /UCR2 {invert} >> \
         /SecondOnly << /UCR2 {invert} >> /FirstOnly << /UCR {identity} >> >>"
    );
    assert_function_entries(
        fixture_with_page_entries(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            &resources,
            "/Both gs 0.2 0.7 0.4 rg 0 0 10 10 re f",
        ),
        "Both",
        &["UCR", "UCR2"],
    );
    let render = |setup: &str| {
        centre_pixel(fixture_with_page_entries(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            &resources,
            &format!("{setup} 0.2 0.7 0.4 rg 0 0 10 10 re f"),
        ))
    };
    let both = render("/Both gs");
    let second = render("/SecondOnly gs");
    let first = render("/FirstOnly gs");
    assert_eq!(
        both, second,
        "the same-dictionary result is UCR2's function"
    );
    assert_ne!(
        both, first,
        "the competing UCR identity function is a negative control"
    );
}
