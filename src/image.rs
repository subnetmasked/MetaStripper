use anyhow::{Context, Result};
use image::ImageFormat;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub fn strip_image_metadata(input_path: &Path, output_path: &Path) -> Result<()> {
    // Read the image
    let img = image::open(input_path)
        .with_context(|| format!("Failed to open image: {}", input_path.display()))?;

    // Determine the output format based on the input file extension
    let format = match input_path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
        Some("png") => ImageFormat::Png,
        Some("gif") => ImageFormat::Gif,
        Some("bmp") => ImageFormat::Bmp,
        Some("tiff") => ImageFormat::Tiff,
        _ => return Err(anyhow::anyhow!("Unsupported image format")),
    };

    // Save the image without metadata
    img.save_with_format(output_path, format)
        .with_context(|| format!("Failed to save image: {}", output_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_strip_image_metadata() {
        // Create a test image with metadata
        let input = NamedTempFile::new().unwrap();
        let output = NamedTempFile::new().unwrap();

        // Create a simple test image
        let img = image::RgbImage::new(100, 100);
        img.save(&input).unwrap();

        // Test stripping metadata
        assert!(strip_image_metadata(input.path(), output.path()).is_ok());
    }
} 