# MetaStripper

A cross-platform command-line tool for removing privacy-sensitive metadata from files before sharing or uploading them.

## Features

- Remove EXIF, GPS, and camera info from image files
- Remove author, creator, and creation/modification time from PDFs
- Remove metadata tags and creation time from video files using ffmpeg
- Support for batch processing of multiple files
- Option to overwrite original files or save cleaned copies
- Progress bar and detailed logging
- Cross-platform support (macOS, Linux)

## Installation

### Prerequisites

- Rust 1.70 or later
- ffmpeg (for video processing)

### Building from Source

```bash
git clone https://github.com/subnetmasked/MetaStripper.git
cd MetaStripper
cargo build --release
```

The compiled binary will be available at `target/release/metastripper`.

### macOS (Homebrew)

```bash
# Add the tap
brew tap subnetmasked/metastripper

# Install the package
brew install metastripper
```

## Usage

Basic usage:
```bash
metastripper input_file.jpg
```

Process multiple files:
```bash
metastripper file1.jpg file2.mp4 file3.jpg
```

Process an entire directory:
```bash
metastripper /path/to/directory
```

Save cleaned files to a different directory:
```bash
metastripper --output-dir /path/to/output input_file.jpg
```

Overwrite original files:
```bash
metastripper --overwrite input_file.jpg
```

Enable verbose logging:
```bash
metastripper --verbose input_file.jpg
```

## Supported File Types

### Images
- JPEG/JPG
- PNG
- GIF
- BMP
- TIFF

### Documents
- PDF

### Videos
- MP4
- MOV
- AVI
- MKV

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU General Public License v3.0 - see the LICENSE file for details.
