// The C ABI of `crates/viewer-ffi`, declared for a .NET host (RFC 0010 step N0, ADR 2054).
//
// Every declaration in this directory is one of `crates/viewer-ffi/include/quorra.h`'s, under the
// header's own name, and nothing here adds behaviour. The header stays the one place an entry point
// is explained: its comments are not repeated here, because two texts drift and no test reads
// prose. `crates/viewer-ffi/tests/header_and_interop_agree.rs` holds the declarations to the
// header instead: each function once, its parameters by name and in order, every type by the rule
// below, every struct field and every constant.
//
// The rule, one C type to one C# type:
//
//   uint8_t int8_t uint16_t int16_t uint32_t int32_t uint64_t int64_t size_t float
//     byte  sbyte  ushort   short   uint     int     ulong    long    nuint  float
//
//   bool, an argument           [MarshalAs(UnmanagedType.U1)] bool: C gives it one byte and the
//                               marshaller's default is four
//   bool in memory              byte: a field, or what a bool * points at, so a struct holding
//                               one stays blittable and its managed layout is the C layout
//   const char *, an argument   string?, marshalled as UTF-8, since every string the header takes
//                               is UTF-8; a null is the library's QUORRA_NULL_ARGUMENT, not a crash
//   any other char *            byte*: an argument is a buffer the caller owns, and a result is a
//                               static string that a marshalled string would try to free
//   an opaque quorra_x *        nint, and each further * a pointer to one
//   a struct, any other T *     the header's struct of that name, or the rule's T, with its *s
//
// Everything is `internal`. A pointer-level surface is what this layer is, not what a window should
// program against, and the layer a window uses is a later step's (RFC 0010 section 3.2).

namespace Quorra.Interop;

internal static partial class Native
{
    // Cargo names the library after the package, and the runtime adds the platform's `lib` and
    // `.so`, or `.dll`, around it. Every function is cdecl, which is Rust's `extern "C"`: 64-bit
    // Windows has no other convention, and saying so keeps a 32-bit process from calling stdcall.
    internal const string Library = "viewer_ffi";
}
