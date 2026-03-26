# `version` Command

`version` prints the current build information for the program.

## Usage

```bash
tdlr version
```

## Output fields

The command prints the following fields:

- `Version`
- `Rustc`
- `Target`

Where:

- `Version` comes from the build-time `TDLR_VERSION`
- `Rustc` is the Rust compiler version used for the build
- `Target` is the current runtime platform in `OS/ARCH` format

## Example

```bash
tdlr version
```

## Reference

| File | Description |
|------|------|
| `build.rs` | Build-time version injection |
| `src/commands/version.rs` | Command implementation |
