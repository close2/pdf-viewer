# RFC 0003's KIO face — a build that is deliberately outside the cargo workspace.
#
# WHY IT IS HERE AND NOT IN `crates/`. RFC 0003 section 7: the shim "lives outside the cargo
# workspace (a `kio/` subdirectory with its own build), so the workspace stays pure Rust and CI
# treats the shim as an optional artefact built where KF6 exists". Nothing in `Cargo.toml` names
# this directory, no build script reaches it, and `cargo build`, `cargo clippy --workspace` and
# `cargo nextest run --workspace` on a machine with no KDE at all are untouched by its existence.
# What builds it is this file, run by hand or by `crates/pdf-vfs-ffi/tests/the_kio_worker.rs`,
# which SKIPS — loudly, saying which package is missing — where the toolchain is absent.
#
#   cmake -S kio -B <build> \
#         -DPDFVFS_INCLUDE_DIR=crates/pdf-vfs-ffi/include \
#         -DPDFVFS_LIBRARY=<target>/debug/libpdf_vfs_ffi.so
#   cmake --build <build>
#
# The two variables are REQUIRED and are not searched for. A `find_library` here would silently
# pick up an installed copy of a different revision, and the whole point of `pdfvfs_abi_check` is
# that such a mismatch is loud — so the build says which library it is linking rather than
# discovering one.
#
# WHAT IT INSTALLS. One `MODULE` plugin, `pdf.so`, into KF6's KIO plugin directory, and nothing
# else. The confined generator `pdf-vfs-worker` is a Rust binary and is installed by whatever
# installs the rest of this project; the plugin looks for it beside the running executable or at
# $PDF_VFS_WORKER, and says so by name when it is absent.

cmake_minimum_required(VERSION 3.20)

project(kio_pdf VERSION 0.1.0 LANGUAGES CXX)

set(QT_MIN_VERSION 6.5.0)
set(KF_MIN_VERSION 6.0.0)

find_package(ECM ${KF_MIN_VERSION} REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})
include(KDEInstallDirs)

find_package(Qt6 ${QT_MIN_VERSION} REQUIRED COMPONENTS Core)
find_package(KF6 ${KF_MIN_VERSION} REQUIRED COMPONENTS KIO)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

set(PDFVFS_INCLUDE_DIR "" CACHE PATH "the directory holding pdf_vfs.h")
set(PDFVFS_LIBRARY "" CACHE FILEPATH "the pdf-vfs-ffi shared library to link against")
if(NOT EXISTS "${PDFVFS_INCLUDE_DIR}/pdf_vfs.h")
    message(FATAL_ERROR "PDFVFS_INCLUDE_DIR must name the directory holding pdf_vfs.h")
endif()
if(NOT EXISTS "${PDFVFS_LIBRARY}")
    message(FATAL_ERROR "PDFVFS_LIBRARY must name libpdf_vfs_ffi.so")
endif()

add_library(kio_pdf MODULE src/pdfworker.cpp)
set_target_properties(kio_pdf PROPERTIES OUTPUT_NAME pdf PREFIX "")
target_include_directories(kio_pdf PRIVATE "${PDFVFS_INCLUDE_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}/src")
target_link_libraries(kio_pdf PRIVATE KF6::KIOCore Qt6::Core "${PDFVFS_LIBRARY}")
# The header states the C ABI's contract and the compiler is asked to hold this file to it: every
# warning here is about the boundary, which is the one place a mistake is invisible from Rust.
target_compile_options(kio_pdf PRIVATE -Wall -Wextra)
install(TARGETS kio_pdf DESTINATION ${KDE_INSTALL_PLUGINDIR}/kf6/kio)

# The harness. A KIO *client* — a QCoreApplication running the same jobs Dolphin runs — so that
# the plugin can be driven on a machine with no KDE session, no Wayland and no D-Bus. It is not
# installed.
option(PDFVFS_BUILD_HARNESS "build the KIO client that drives the plugin" ON)
if(PDFVFS_BUILD_HARNESS)
    add_executable(drive_the_worker test/drive_the_worker.cpp)
    target_link_libraries(drive_the_worker PRIVATE KF6::KIOCore Qt6::Core)
    target_compile_options(drive_the_worker PRIVATE -Wall -Wextra)
endif()
