//! ISO 32000-2 §10.8.3's simulated press is refused by name on this backend (ADR 1317).
//!
//! A separated page is a plane per ink put together per pixel after every plane is drawn, and a
//! Vello scene renders one raster with no pass over its result, so the refusal is what sends the
//! frame to the CPU backend. It is asserted by the clause it names rather than by the pair's own
//! refusal, which a separated page also carries and which would describe a different page.

use std::sync::Arc;

use pdf_render::{
    BlendingSpace, ColourCube, DisplayList, Rasterizer, Size, SpotColourant, SpotSeparation,
    TargetSpec,
};
use render_gpu::{GpuRasterError, GpuRasterizer};

#[test]
fn a_separated_page_is_refused_by_name() {
    let cube = || {
        let grid: Vec<[f32; 3]> = (0..8_u8)
            .map(|corner| {
                [
                    f32::from(corner & 1),
                    f32::from(corner >> 1 & 1),
                    f32::from(corner >> 2),
                ]
            })
            .collect();
        ColourCube::new(
            Arc::from(vec![[0.0; 3], [1.0; 3]]),
            2,
            Arc::from(grid),
            Arc::from(vec![0.0, 1.0]),
        )
        .expect("two curves and eight corners")
    };
    let page = Size::new(10.0, 10.0);
    let colourant = SpotColourant::new(Arc::from(b"Spot".as_slice()), Arc::from(vec![[1.0; 3]; 2]))
        .expect("two samples");
    let separation = SpotSeparation::new(
        vec![colourant],
        vec![DisplayList::new(page)],
        cube(),
        cube(),
    )
    .expect("one colourant, one plane");
    let space = BlendingSpace::new(2, Arc::from(vec![[1.0; 3]; 16])).expect("sixteen corners");
    let mut list = DisplayList::new(page);
    list.set_separated(space, DisplayList::new(page), separation);
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a small target");

    let mut gpu = GpuRasterizer::new_headless().unwrap_or_else(|error| {
        panic!(
            "no GPU adapter available: {error}\n\
             Install a Vulkan driver (mesa-vulkan-drivers for a software one). These tests do not \
             skip, because a skipped GPU suite reports success while verifying nothing."
        )
    });
    let refused = gpu.rasterize(&list, target);
    assert!(
        matches!(&refused, Err(GpuRasterError::UnsupportedCommand(why)) if why.contains("§10.8.3")),
        "{refused:?}"
    );
}
