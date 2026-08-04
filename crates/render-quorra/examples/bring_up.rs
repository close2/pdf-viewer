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
//! cargo run --release -p render-quorra --example bring_up -- [all|vulkan|gl]
//! ```
//!
//! and compare processes, not lines.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::Instant;

use quorra_gpu::wgpu;

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1e3
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "all".to_owned());
    let backends = match which.as_str() {
        "all" => wgpu::Backends::all(),
        "vulkan" => wgpu::Backends::VULKAN,
        "gl" => wgpu::Backends::GL,
        other => panic!("unknown backend set {other:?}; try all, vulkan or gl"),
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
