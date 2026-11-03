//! [`super::gpu_lock`] installed as `raster_gpu`'s device-section hook (ADR 2664).
//!
//! `raster_gpu` says when it begins and ends work on the device — a bring-up, a frame, an upload —
//! and nothing else; this hook turns each hardware section into a hold on the shared adapter's lock.
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

use raster_gpu::{DeviceWork, Section, SectionHook};

/// The hook: a hold for each section on a hardware adapter.
struct LockEachSection;

impl SectionHook for LockEachSection {
    fn begin(&self, section: Section) {
        if section.device_type != raster_gpu::wgpu::DeviceType::Cpu {
            super::gpu_lock::enter(label(section.work));
        }
    }

    fn end(&self, section: Section) {
        if section.device_type != raster_gpu::wgpu::DeviceType::Cpu {
            super::gpu_lock::leave();
        }
    }
}

fn label(work: DeviceWork) -> &'static str {
    match work {
        DeviceWork::BringUp => "bring-up",
        DeviceWork::Frame => "frame",
        DeviceWork::Upload => "upload",
    }
}

/// Installs the hook for this process, once; a second call, or a hook installed by someone else,
/// leaves the installed one in place.
pub(crate) fn install() {
    if super::gpu_lock::lock_file().is_some() && !raster_gpu::section_hook_installed() {
        drop(raster_gpu::install_section_hook(Box::new(LockEachSection)));
    }
}
