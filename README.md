# Introduction
[![x86-windows](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-windows.yml/badge.svg)](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-windows.yml)
[![x86-linux](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-linux.yml/badge.svg)](https://github.com/williamyang98/wgpu_terminal/actions/workflows/x86-linux.yml)

Basic terminal emulator written in Rust. 

This is a hobby project to explore how terminal emulators are implemented and should not be used in production.

## Instructions
- Build: ```cargo build -r```
- Run: ```cargo run -r```
- Show help: ```cargo run -r -- --help```
- Run with options (example): ```WGPU_BACKEND=gl RUST_LOG=info cargo run -r -- bash.exe```

## Features
- Basic handling of VT100+ codes and UTF8 parsing
- Scrollback buffer
- Works with Windows conpty and Linux pty
- Wgpu full 24-bit colour renderer
- Custom fonts
- Launch process directly without operating system pseudoterminal to benchmark emulator code directly

## Gallery
![Screenshot](/docs/screenshot.png)
