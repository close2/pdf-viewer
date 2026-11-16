Status: complete
Given: 2026-11-16, in conversation — transcribed by the round
Owes: re-run marks_across_views and fitted_dashes_across_views 450 times each against the installed libvulkan1 1.4.363.0-1 with InstanceFlags::DEBUG restored, and if neither crashes, reverse ADR 2903's trade by a new ADR so debug builds regain GPU object names (a render-raster / GPU round)

> I have already installed a newer version.  Please check.

Reading: adopts option (a) — `libvulkan1` and `libvulkan-dev` `1.4.363.0-1` (`/usr/lib/x86_64-linux-gnu/libvulkan.so.1.4.363`, `Vulkan Instance Version: 1.4.363`, well past `v1.4.345`'s fix in `KhronosGroup/Vulkan-Loader` PR #1866 / commit `1338b8ee`) are now installed on the VM. A round now owes re-running `marks_across_views` and `fitted_dashes_across_views` 450 times each with `InstanceFlags::DEBUG` restored and, once clean, reversing ADR 2903's trade by a new ADR.
