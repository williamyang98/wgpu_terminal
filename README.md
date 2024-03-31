# Introduction
[![x86-windows](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-windows.yml/badge.svg)](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-windows.yml)

Basic terminal emulator written for Windows in Rust. This is a hobby project to explore how terminal emulators are implemented and should not be used in production.

## Instructions
- Build: ```cargo build -r```
- Run: ```cargo run -r```
- Run with options: ```WGPU_BACKEND=gl RUST_LOG=info cargo run -r -- bash.exe```
- Show help: ```cargo run -r -- --help```

## Features
- Basic handling of VT100+ codes and UTF8 parsing
- Scrollback buffer
- Interop with ConPty for Windows pseudo-terminal
- Wgpu full 24-bit colour renderer
- Custom fonts
- Launch process without Conpty (very slow) to benchmark emulator code properly

## Gallery
![Screenshot](/docs/screenshot.png)
