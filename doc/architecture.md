# Architecture

The emulator is split into two crates: glvs and glvs-core. The latter contains the core
emulation library and is `no_std`, which means it doesn't depend on the Rust standard
library.

## CPU

It currently implements all official opcodes and a few unofficial ones, for a total of
172. It is tested extensively using [single step tests][sst].

It does not require the Rust standard library or dynamic allocation.

[sst]: https://github.com/SingleStepTests/65x02/tree/main/nes6502
