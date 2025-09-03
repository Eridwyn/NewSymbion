# Building for Windows

To build the Windows binary:

1. On Windows machine with Rust installed:
```bash
cargo build --release
```

2. Rename the output:
```bash
copy target\release\symbion-agent-host.exe symbion-agent-host-windows-x64.exe
```

3. Or use cross-compilation (requires setup):
```bash
# Install Windows target
rustup target add x86_64-pc-windows-gnu

# Install mingw-w64 toolchain
sudo apt install gcc-mingw-w64-x86-64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```
