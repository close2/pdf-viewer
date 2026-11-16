//! Table378's eight writing modes and section14.8.5.4.5's allocation rectangle.
//! Coordinates are PDF default user space: y increases upward. The unequal spacing
//! distinguishes before from after; literal rectangles cover all four block directions.

#![expect(
    clippy::float_cmp,
    reason = "small integer coordinates and spacing are exactly representable"
)]

use pdf_model::structure::{BlockSpacing, WritingMode, allocation_rectangle};

#[test]
fn all_eight_writing_modes_adjust_the_declared_before_and_after_edges() {
    let cases = [
        ("LrTb", [10.0, 17.0, 30.0, 42.0]),
        ("RlTb", [10.0, 17.0, 30.0, 42.0]),
        ("TbRl", [7.0, 20.0, 32.0, 40.0]),
        ("TbLr", [8.0, 20.0, 33.0, 40.0]),
        ("LrBt", [10.0, 18.0, 30.0, 43.0]),
        ("RlBt", [10.0, 18.0, 30.0, 43.0]),
        ("BtRl", [7.0, 20.0, 32.0, 40.0]),
        ("BtLr", [8.0, 20.0, 33.0, 40.0]),
    ];
    for (name, expected) in cases {
        let mode = WritingMode::read(name.as_bytes()).expect("Table378 name");
        let actual = allocation_rectangle(
            [10.0, 20.0, 30.0, 40.0],
            BlockSpacing {
                before: 2.0,
                after: 3.0,
            },
            mode,
        );
        assert_eq!(
            actual, expected,
            "literal default-user-space edges for {name}"
        );
    }
}
