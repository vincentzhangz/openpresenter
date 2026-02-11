# OpenPresenter

A modern presentation software built with Rust, featuring NDI video output for professional live production workflows. Similar to ProPresenter, but open source and cross-platform.

> ⚠️ **Early Stage Development**: This project is in active development. APIs and features may change without notice. Breaking changes should be expected.

## Features

### Currently Implemented ✅

- **Modern UI**: Built with [iced](https://iced.rs/) using the Elm architecture for predictable state management
- **Presentation Library**: SQLite-backed library with full-text search support (FTS5)
- **Database Management**: Create, edit, and organize presentations with CRUD operations
- **NDI Integration**: FFI bindings to NDI SDK v6 for professional video output
- **GPU Acceleration**: Hardware-accelerated rendering with [wgpu](https://wgpu.rs/) (Metal/Vulkan/DX12)
- **Cross-Platform**: Runs on macOS, with Windows and Linux support planned

### In Development 🚧

- Video frame decoding and playback (FFmpeg integration)
- GPU-accelerated text rendering with glyphon
- Background images and video support
- Slide compositor with layer blending
- Transition effects (fade, cut, slide)
- Real-time NDI video output

### Planned 📋

- Full-text search for songs and lyrics
- Import from ProPresenter and OpenLyrics formats
- Media library with thumbnail generation
- Keyboard shortcuts and hotkeys
- Stage display mode
- Multi-monitor support
- Recording and export capabilities

## Prerequisites

### macOS
- Xcode Command Line Tools
- Rust 1.93+ (edition 2024)
- [Homebrew](https://brew.sh/)
- FFmpeg 8.0.1+ (`brew install ffmpeg`)
- pkg-config (`brew install pkgconf`)
- [NDI SDK for Apple](https://ndi.video/for-developers/ndi-sdk/) v6.0+

### Windows
Windows support is planned. Required dependencies:
- Visual Studio Build Tools
- Rust 1.93+
- FFmpeg
- NDI SDK for Windows

### Linux
Linux support is planned. Required dependencies:
- GCC/Clang
- Rust 1.93+
- FFmpeg
- NDI SDK for Linux

## Installation

```bash
# Clone the repository
git clone https://github.com/vincentzhangz/openpresenter.git
cd openpresenter

# Install system dependencies (macOS)
brew install ffmpeg pkgconf

# Build the project
cargo build --release

# Run the application
cargo run --release
```

### NDI SDK Setup

1. Download the [NDI SDK](https://ndi.video/for-developers/ndi-sdk/) for your platform
2. Install to the default location:
   - **macOS**: `/Library/NDI SDK for Apple/lib/macOS/libndi.dylib`
   - **Windows**: `C:\Program Files\NDI\NDI SDK\Lib\x64\Processing.NDI.Lib.x64.dll`
   - **Linux**: `/usr/lib/libndi.so`
3. The application will dynamically link to the NDI library at runtime

### FFmpeg Configuration

On macOS, the build system uses Homebrew's FFmpeg installation. Configuration is in `.cargo/config.toml`:
```toml
[env]
FFMPEG_DIR = "/opt/homebrew/opt/ffmpeg"
CPATH = "/opt/homebrew/opt/ffmpeg/include"
```

For custom FFmpeg installations, update these paths accordingly.

## Configuration

Application data is stored in platform-specific directories:
- **macOS**: `~/Library/Application Support/openpresenter/`
- **Windows**: `%APPDATA%\openpresenter\`
- **Linux**: `~/.config/openpresenter/`

The database file (`library.db`) and configuration are automatically created on first run.

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Run in development mode
cargo run

# Run tests
cargo test
```

### Code Quality

```bash
# Check for errors
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Sort dependencies (requires cargo-sort)
cargo sort -w
```

### Project Guidelines

- Uses Rust edition 2024
- Follows Rust idioms from `.github/instructions/rust.instructions.md`
- All code must pass `cargo clippy` without warnings
- Format with `cargo fmt` before committing

## Contributing

Contributions are welcome! This project is in active development.

### How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests and linting (`cargo test && cargo clippy`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Development Priorities

Current focus areas for contributions:
- Video decoding implementation with FFmpeg
- GPU text rendering with glyphon
- Slide compositor and layer blending
- NDI output loop implementation
- Cross-platform testing and compatibility

## Resources

- [iced Documentation](https://docs.rs/iced/) - UI framework
- [wgpu Guide](https://sotrh.github.io/learn-wgpu/) - GPU rendering
- [FFmpeg Documentation](https://ffmpeg.org/documentation.html) - Media processing
- [NDI SDK Download](https://ndi.video/for-developers/ndi-sdk/) - Video streaming

## License

TBD
