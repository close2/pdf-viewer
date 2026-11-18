//! Public display-list regression for Type 5's inherited per-vertex byte alignment.
//! The low-bit stream and its direct 8-bit control state the same four vertices.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a valid exact-xref fixture and bounded vertex comparisons must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Corners, Paint, ShadingKind};
use pdf_syntax::Document;

fn fixture(bits_per_coordinate: u8, bits_per_component: u8, samples: &str) -> Vec<u8> {
    let shading = format!(
        "<< /ShadingType 5 /ColorSpace /DeviceGray /VerticesPerRow 2 \
         /BitsPerCoordinate {bits_per_coordinate} /BitsPerComponent {bits_per_component} \
         /Decode [0 100 0 100 0 1] /Filter /ASCIIHexDecode /Length {} >>\nstream\n{samples}\nendstream",
        samples.len()
    );
    let content = "/Sh0 sh";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n{shading}\nendobj\n",
        content.len().saturating_add(1)
    );
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn triangle_vertices(
    samples: &str,
    coordinate_bits: u8,
    component_bits: u8,
) -> Vec<[([f32; 2], f32); 3]> {
    let document = Document::open(fixture(coordinate_bits, component_bits, samples))
        .expect("complete PDF fixture");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "complete valid lattice: {:?}",
        interpretation.unsupported
    );
    let shading = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Fill {
                paint: Paint::Shading(shading),
                ..
            } => Some(shading),
            _ => None,
        })
        .expect("the page paints its mesh");
    let ShadingKind::Mesh {
        triangles,
        patches: None,
        ..
    } = shading.kind.as_ref()
    else {
        panic!("Type 5 is represented as mesh triangles");
    };
    triangles
        .iter()
        .map(|triangle| {
            let Corners::Colours(colours) = triangle.corners else {
                panic!("direct DeviceGray vertices carry decoded colors");
            };
            std::array::from_fn(|i| {
                let point = triangle.points[i];
                let colour = colours[i];
                assert!((colour.r - colour.g).abs() < 1e-6);
                assert!((colour.g - colour.b).abs() < 1e-6);
                ([point.x, point.y], colour.r)
            })
        })
        .collect()
}

fn assert_same_source_vertices(triangles: &[[([f32; 2], f32); 3]]) {
    let expected = [
        [([0.0, 0.0], 0.0), ([100.0, 0.0], 1.0), ([0.0, 100.0], 1.0)],
        [
            ([100.0, 0.0], 1.0),
            ([0.0, 100.0], 1.0),
            ([100.0, 100.0], 0.0),
        ],
    ];
    assert_eq!(triangles.len(), expected.len());
    for expected_triangle in expected {
        let count = triangles
            .iter()
            .filter(|triangle| {
                expected_triangle.iter().all(|(point, gray)| {
                    triangle.iter().any(|(actual_point, actual_gray)| {
                        (actual_point[0] - point[0]).abs() < 1e-5
                            && (actual_point[1] - point[1]).abs() < 1e-5
                            && (actual_gray - gray).abs() < 1e-6
                    })
                })
            })
            .count();
        assert_eq!(
            count, 1,
            "one literal Table82 triangle {expected_triangle:?}"
        );
    }
}

#[test]
fn type5_low_bit_vertices_pad_to_bytes_like_the_8bit_control() {
    // Type 5 shares Type 4's per-vertex format/padding rule (Table 82's final cross-reference).
    // Each record here has 2+2+1=5 significant bits and three ignored pad bits. The byte values
    // encode (x,y,gray): (0,0,0), (3,0,1), (0,3,1), (3,3,0); Decode maps 2-bit coordinates
    // 0/3 to 0/100 and 1-bit gray to 0/1. A reader that consumes a continuous 20-bit stream
    // rather than aligning each vertex will not recover the literal four vertices.
    // Positive control: the identical row-major lattice with one whole byte per field has no
    // per-vertex pad bits and must produce the same two Table 82 triplets and colors.
    let byte_aligned = triangle_vertices("00 00 00 FF 00 FF 00 FF FF FF FF 00 >", 8, 8);
    assert_same_source_vertices(&byte_aligned);

    let padded = triangle_vertices("00 C8 38 F0 >", 2, 1);
    assert_same_source_vertices(&padded);

    // Ignored padding is not required to be zero: set all three trailing bits.
    let nonzero_padding = triangle_vertices("07 CF 3F F7 >", 2, 1);
    assert_same_source_vertices(&nonzero_padding);
}

fn endpoint_lattice_samples(coordinate_bits: u8, component_bits: u8) -> String {
    let mut hex = String::new();
    for (x, y, gray) in [
        ('0', '0', '0'),
        ('1', '0', '1'),
        ('0', '1', '1'),
        ('1', '1', '0'),
    ] {
        // Endpoint samples are literal all-zero/all-one bit strings. This fixture builder
        // does not call a production bit writer or reproduce its integer-width arithmetic.
        let mut record = format!(
            "{}{}{}",
            x.to_string().repeat(usize::from(coordinate_bits)),
            y.to_string().repeat(usize::from(coordinate_bits)),
            gray.to_string().repeat(usize::from(component_bits))
        );
        while !record.len().is_multiple_of(8) {
            record.push('1');
        }
        for byte in record.as_bytes().as_chunks::<8>().0 {
            let binary = std::str::from_utf8(byte).expect("ASCII bit string");
            let value = u8::from_str_radix(binary, 2).expect("exactly eight binary digits");
            write!(hex, "{value:02X} ").expect("write hexadecimal fixture");
        }
    }
    hex.push('>');
    hex
}

#[test]
fn type5_accepts_every_table82_coordinate_and_component_width() {
    // Table82 lists eight coordinate widths and six component widths. The same four
    // Decode endpoints must survive every permitted pair, including 32-bit coordinates
    // and records crossing byte boundaries. The fixed-byte test above independently
    // anchors the literal hex encoding used here.
    for coordinate_bits in [1, 2, 4, 8, 12, 16, 24, 32] {
        for component_bits in [1, 2, 4, 8, 12, 16] {
            let samples = endpoint_lattice_samples(coordinate_bits, component_bits);
            let triangles = triangle_vertices(&samples, coordinate_bits, component_bits);
            assert_same_source_vertices(&triangles);
        }
    }
}
