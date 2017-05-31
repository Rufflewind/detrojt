# Deserializeable trait objects

A dirty hack created to support deserialization of arbitrary trait objects.

This is a proof-of-concept created in response to [rust-lang/rfcs#668](https://github.com/rust-lang/rfcs/issues/668) as well as Dmitry Gordon's question [*How can deserialization of polymorphic trait objects be added in Rust if at all?*](https://stackoverflow.com/q/44231020/440302) ([related Rust Forum post](https://users.rust-lang.org/t/reflection-in-rust/11069)).

**Deserializing untrustworthy data may cause arbitrary code execution.**  The library has some checks built in to make it slightly hard to exploit, but I can't guarantee this library is 100% safe to use in the face of a serious attacker.  It would be prudent to either secure your communications or at the very least sign the data with an unforgeable signature.

## Assumptions

In order to make this work, we need to assume a few things:

  - All vtables are mapped to a contiguous block of memory
  - Vtables located at fixed positions relative to each other
  - Vtables contain `{ destructor, size, alignment, ... }`
  - A POSIX system with `/dev/random` (you can probably port this to other platforms without much difficulty)
  - 64-bit pointers (not entirely necessary, but 32-bit pointers would make it easier to exploit)
  - Nightly Rust compiler (not technically necessary, but we're assuming a lot about Rust's internal vtable implementation anyway ...)

If [Rust adds support for `#[repr(align = "N")]`](https://github.com/rust-lang/rust/issues/33626), it may be possible to use a custom alignment as a secondary sanity check.
