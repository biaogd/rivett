# Rivett

A lightweight SSH client built in Rust with a fast, unified workflow.

## Features

- Fast terminal renderer (GPU path with CPU fallback)
- SFTP panel with file operations and breadcrumb navigation
- SSH key management (import + fingerprint parsing)
- Per-session port forwarding UI and status
- IME input support (Chinese input)

## Status

This project is under active development. Expect changes and occasional rough edges.

## Cross-platform

The app is currently focused on macOS, but the architecture is intended to be cross-platform. Windows and Linux support are planned.

## Built with

- [iced](https://github.com/iced-rs/iced)
- [alacritty_terminal](https://github.com/alacritty/alacritty)
- [russh](https://github.com/warp-tech/russh)

## Build

```bash
cargo run
```

Release build:

```bash
cargo build -r
```

## Packaging (macOS DMG)

```bash
./scripts/macos/build_dmg.sh
```

## Contributing

Issues and PRs are welcome.

## License

TBD
