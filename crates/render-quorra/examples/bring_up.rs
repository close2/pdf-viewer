//! Where the graphics device's cold bring-up time actually goes.
//!
//! `CLAUDE.md` puts GPU bring-up on the critical path by choice — page one goes to the
//! device — and makes what it costs a number to keep small. `--trace` prints the three
//! parts quorra reports; this example takes the *first* of them apart, because a host
//! cannot: `StartupTimings::adapter_enumeration` is measured from before
//! `wgpu::Instance::new`, so instance creation, surface creation and the adapter request
//! are one figure.
//!
//! **One measurement per process, deliberately.** Everything here loads drivers, and a
//! second instance in the same process is measured with the loader already warm — the
//! first version of this example created two and reported 26 ms against 4 ms for the same
//! work in the other order. Run it as
//!
//! ```sh
//! cargo run --release -p render-quorra --example bring_up -- [all|vulkan|gl|overlap]
//! ```
//!
//! and compare processes, not lines. `overlap` asks the other question — whether the instance's
//! creation can be hidden behind the document's, which is `doc/QUORRA_FEEDBACK.md` section 8.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::Instant;

use quorra_gpu::wgpu;

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1e3
}

/// Opens the largest document in the tree and reads what a launch reads before it has a window.
///
/// The main thread's half of the `overlap` mode: `viewer_core::Open::around`'s first two steps,
/// which is what a launch is doing while nothing is asking the graphics stack anything.
fn document_work() -> usize {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is in doc/");
    let document = pdf_syntax::Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let outline = pdf_model::outline::Outline::read(&document, &pages);
    pages.len().saturating_add(outline.items.len())
}

/// Does the instance's creation overlap the work a launch does before it has a window?
///
/// The question `doc/QUORRA_FEEDBACK.md` section 8 asks, measured rather than argued: an instance needs
/// no window and no surface, so a host that could create one on a thread of its own would pay
/// `max(instance, document)` instead of `instance + document`. It cannot today —
/// `Device::for_surface` creates the instance itself — so this measures what that would be worth
/// using `wgpu` directly, which is the same call quorra makes.
fn overlap() {
    let serial = Instant::now();
    let read = document_work();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let one_after_the_other = ms(serial);
    drop(instance);

    let together = Instant::now();
    let creating = std::thread::spawn(|| {
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
    });
    let read_again = document_work();
    let instance = creating.join().expect("the instance thread");
    let side_by_side = ms(together);
    drop(instance);

    assert_eq!(read, read_again, "the same document both times");
    println!("document then instance   {one_after_the_other:8.3} ms");
    println!("both at once             {side_by_side:8.3} ms");
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "all".to_owned());
    if which == "overlap" {
        overlap();
        return;
    }
    let backends = match which.as_str() {
        "all" => wgpu::Backends::all(),
        "vulkan" => wgpu::Backends::VULKAN,
        "gl" => wgpu::Backends::GL,
        other => panic!("unknown backend set {other:?}; try all, vulkan, gl or overlap"),
    };

    let whole = Instant::now();
    let started = Instant::now();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let instance_creation = ms(started);

    let started = Instant::now();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("an adapter");
    let request = ms(started);
    let info = adapter.get_info();

    let started = Instant::now();
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bring_up"),
        required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .expect("a device");
    let device_creation = ms(started);
    let total = ms(whole);
    drop(device);

    println!("backends {which}, chose {} ({:?})", info.name, info.backend);
    println!("  instance creation   {instance_creation:8.3} ms");
    println!("  request_adapter     {request:8.3} ms");
    println!("  request_device      {device_creation:8.3} ms");
    println!("  usable device in    {total:8.3} ms");
}
