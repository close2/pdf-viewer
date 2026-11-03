//! The backends a device test enumerates: the set `WGPU_BACKEND` names, and every backend only
//! where it names none (ADR 2665).
//!
//! A test is a host, and raster's ADR 0017 leaves the environment to the host: the library reads
//! none, and a host that wants `WGPU_BACKEND` honoured says so in one line. The device tests say
//! it here, because the variable is how this machine selects lavapipe (ADR 2663) — with
//! `VK_ICD_FILENAMES` naming lavapipe, the Vulkan half holds no hardware, but an instance over
//! every backend still loads OpenGL, whose driver is the shared card's, and a high-performance
//! request or an enumeration of every adapter then reaches the card through it (ADR 2665).
//!
//! Included by `#[path]` beside `gpu_lock.rs`; it names `raster_gpu::wgpu`, so it compiles in any
//! crate that depends on `raster-gpu`.

#![allow(
    dead_code,
    reason = "each including test uses the part it needs, and a #[path] module is compiled per crate"
)]
#![allow(
    unused_qualifications,
    reason = "a crate that depends on `wgpu` itself sees `raster_gpu::wgpu` as the same path, and \
              this module names it so that a crate that does not still compiles"
)]

/// The set `WGPU_BACKEND` names, or every backend where it names none.
pub(crate) fn from_environment() -> raster_gpu::wgpu::Backends {
    raster_gpu::wgpu::Backends::from_env().unwrap_or(raster_gpu::wgpu::Backends::all())
}

/// An instance over [`from_environment`]'s backends, made the way raster makes its own.
pub(crate) fn instance() -> raster_gpu::wgpu::Instance {
    raster_gpu::create_instance_with(from_environment())
}
