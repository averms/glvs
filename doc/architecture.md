# Architecture

The emulator is split into two crates: glvs and glvs-core. The latter contains the core
emulation library and is `no_std`, which means it doesn't depend on the Rust standard
library. The former is the frontend, built using SDL3.

## CPU

It currently implements all official opcodes and a few unofficial ones, for a total of
172. It is tested extensively using [single step tests][sst].

It avoids dynamic allocation.

[sst]: https://github.com/SingleStepTests/65x02/tree/main/nes6502

## PPU
