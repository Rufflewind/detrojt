# Deserializeable trait objects

[![Documentation](https://docs.rs/detrojt/badge.svg)](https://docs.rs/detrojt)
[![Crates.io](https://img.shields.io/crates/v/detrojt.svg)](https://crates.io/crates/detrojt)

[Documentation for the `master` branch](https://rufflewind.com/detrojt)

A dirty hack to support deserialization of arbitrary trait objects.

This is a proof-of-concept created in response to [rust-lang/rfcs#668](https://github.com/rust-lang/rfcs/issues/668) as well as Dmitry Gordon's question [*How can deserialization of polymorphic trait objects be added in Rust if at all?*](https://stackoverflow.com/q/44231020/440302) ([related Rust Forum post](https://users.rust-lang.org/t/reflection-in-rust/11069)).

## Caveat emptor

**Deserialization may cause arbitrary code execution.**  The library has some sanity checks to make it hard to accidentally screw up, but there's no guarantee that this library is safe against a malicious attacker.

Even for trusted data, deserializing may cause undefined behavior on platforms and configurations that violate any of the following assumptions:

  - The data being deserialized was originally serialized by the exact same executable built under identical conditions (architecture, optimization levels, other compiler flags, etc)
  - All vtables are mapped to a single contiguous block of memory, located at fixed positions relative to each other (same for every execution)
  - Trait objects have the layout `{ data: *mut _, vtable: *mut _ }`
  - Vtables have the layout `{ destructor: fn(_), size: usize, alignment: usize, ... }`
  - A POSIX system with either `/dev/random` or `/dev/null` (it shouldn't be too hard to port this to other platforms)
  - 64-bit pointers (not entirely necessary, but 32-bit pointers would make it easier to exploit)

If [Rust adds support for `#[repr(align = "N")]`](https://github.com/rust-lang/rust/issues/33626), it may be possible to use a custom alignment as a secondary sanity check.
