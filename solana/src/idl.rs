// idl.rs - IDL export marker
//
// This module is intentionally minimal. Anchor's procedural macros
// automatically extract IDL information from your #[program], #[account],
// #[event], and #[error_code] decorators. This file exists as a compile-time
// signal that IDL generation is expected.
//
// To generate the IDL:
//   cargo build --features idl-build
//   # IDL appears in target/idl/habitat_settlement_program.json
//
// The IDL is checked in to version control so Go code generators can use it
// without rebuilding the Rust program.

// Marker for Anchor IDL generation. The macros in lib.rs handle the rest.
// No runtime code needed here.
