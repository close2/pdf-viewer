//! [`super::gpu_lock`] installed as `raster_gpu`'s device-section hook (ADR 2664).
//!
//! `raster_gpu` says when it begins and ends work on the device — a bring-up, a frame, an upload,
//! and around them the device's whole life — and nothing else; this hook turns each hardware
//! section into a hold on the shared adapter's lock, so that a device holds it from its bring-up to
//! its drop (ADR 2675), and times every section but the lifetime against the hang bound on its own.
//! A test calls [`install`] before its first device, and every device it makes is covered from then
//! on, however many it makes and on whatever thread. Included by `#[path]` beside `gpu_lock.rs`, in
//! a crate that depends on `raster-gpu` (directly or as `render_raster`'s re-export).

#![allow(
    dead_code,
    reason = "each including test uses the part it needs, and a #[path] module is compiled per crate"
)]
#![allow(
    unused_qualifications,
    reason = "a crate that depends on `wgpu` itself sees `raster_gpu::wgpu` as the same path, and the hook names it so that a crate that does not still compiles"
)]

use std::cell::RefCell;
use std::time::Instant;

use raster_gpu::{DeviceWork, Section, SectionHook};

thread_local! {
    /// When each open timed section on this thread began, innermost last: `raster_gpu` ends a
    /// bring-up, a frame or an upload on the thread that began it, so they nest per thread.
    static BEGAN: RefCell<Vec<Instant>> = const { RefCell::new(Vec::new()) };
}

/// The hook: a hold for each section on a hardware adapter.
struct LockEachSection;

impl SectionHook for LockEachSection {
    fn begin(&self, section: Section) {
        if section.work == DeviceWork::BringUp {
            super::gpu_lock::note_adapter(&format!("{:?}", section.device_type));
        }
        if section.device_type != raster_gpu::wgpu::DeviceType::Cpu {
            super::gpu_lock::enter(label(section.work));
            if section.work != DeviceWork::Lifetime {
                BEGAN.with(|began| began.borrow_mut().push(Instant::now()));
            }
        }
    }

    fn end(&self, section: Section) {
        if section.device_type != raster_gpu::wgpu::DeviceType::Cpu {
            super::gpu_lock::leave();
            if section.work != DeviceWork::Lifetime
                && let Some(began) = BEGAN.with(|began| began.borrow_mut().pop())
            {
                super::gpu_lock::ended(label(section.work), began);
            }
        }
    }
}

fn label(work: DeviceWork) -> &'static str {
    match work {
        DeviceWork::BringUp => "bring-up",
        DeviceWork::Frame => "frame",
        DeviceWork::Upload => "upload",
        DeviceWork::Lifetime => "device",
    }
}

/// Installs the hook for this process, once; a second call, or a hook installed by someone else,
/// leaves the installed one in place.
pub(crate) fn install() {
    if super::gpu_lock::lock_file().is_some() && !raster_gpu::section_hook_installed() {
        drop(raster_gpu::install_section_hook(Box::new(LockEachSection)));
    }
}
