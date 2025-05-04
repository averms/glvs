# glvs

An emulator for Nintendo Entertainment System (NES).

![A screenshot of the first level of Ice
Climber](https://github.com/user-attachments/assets/9598143e-fb3a-4863-9d21-07635530224d)

## Controls

- Arrow keys for the D-pad
- <kbd>A</kbd> is the A button
- <kbd>S</kbd> is the B button
- <kbd>Q</kbd> is Select
- <kbd>W</kbd> is Start

## Running tests

After cloning the repo:

```
cargo xtask fetch-tests
cargo test
```

Extracting the tests depends on tar(1) and curl(1), which are shipped with all major
OSes these days.

## Internals

For writing on the internals and architecture of the emulator, see
[./doc/architecture.md](./doc/architecture.md).
