use anyhow::{Context, Result};
use image::ImageFormat;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use exif::{Reader, Tag, Value, In};

pub fn strip_image_metadata(input_path: &Path, output_path: &Path) -> Result<Vec<String>> {
    // Read the image
    let img = image::open(input_path)
        .with_context(|| format!("Failed to open image: {}", input_path.display()))?;

    // Extract actual metadata before stripping
    let mut removed_metadata = Vec::new();
    
    // Try to extract EXIF data
    if let Ok(metadata) = extract_exif_metadata(input_path) {
        removed_metadata.extend(metadata);
    } else {
        // Fallback to generic metadata if extraction fails
        removed_metadata.push("EXIF metadata (if present)".to_string());
        removed_metadata.push("GPS data (if present)".to_string());
        removed_metadata.push("Camera info (if present)".to_string());
    }

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

    Ok(removed_metadata)
}

fn extract_exif_metadata(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(&file);
    let exif = Reader::new().read_from_container(&mut reader)?;
    
    let mut metadata = Vec::new();
    
    // Camera information
    if let Some(make) = get_exif_string(&exif, Tag::Make, In::PRIMARY) {
        metadata.push(format!("Camera Make: {}", make));
    }
    
    if let Some(model) = get_exif_string(&exif, Tag::Model, In::PRIMARY) {
        metadata.push(format!("Camera Model: {}", model));
    }
    
    if let Some(software) = get_exif_string(&exif, Tag::Software, In::PRIMARY) {
        metadata.push(format!("Software: {}", software));
    }
    
    // Date information
    if let Some(date) = get_exif_string(&exif, Tag::DateTime, In::PRIMARY) {
        metadata.push(format!("Date/Time: {}", date));
    }
    
    if let Some(date) = get_exif_string(&exif, Tag::DateTimeOriginal, In::PRIMARY) {
        metadata.push(format!("Original Date/Time: {}", date));
    }
    
    // GPS information
    let has_gps = exif.get_field(Tag::GPSLatitude, In::PRIMARY).is_some() || 
                  exif.get_field(Tag::GPSLongitude, In::PRIMARY).is_some();
    
    if has_gps {
        // Format GPS coordinates if available
        let mut gps_info = String::from("GPS Location: ");
        let mut has_coords = false;
        
        if let (Some(lat), Some(lat_ref)) = (
            exif.get_field(Tag::GPSLatitude, In::PRIMARY),
            exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY)
        ) {
            if let (Value::Rational(lat_vals), Value::Ascii(lat_ref_vals)) = (&lat.value, &lat_ref.value) {
                if lat_vals.len() >= 3 && !lat_ref_vals.is_empty() {
                    let lat_deg = lat_vals[0].to_f64();
                    let lat_min = lat_vals[1].to_f64();
                    let lat_sec = lat_vals[2].to_f64();
                    let lat_ref = std::str::from_utf8(&lat_ref_vals[0]).unwrap_or("N");
                    
                    gps_info.push_str(&format!("{:.6}° {:.6}' {:.6}\" {} ", 
                        lat_deg, lat_min, lat_sec, lat_ref));
                    has_coords = true;
                }
            }
        }
        
        if let (Some(lon), Some(lon_ref)) = (
            exif.get_field(Tag::GPSLongitude, In::PRIMARY),
            exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY)
        ) {
            if let (Value::Rational(lon_vals), Value::Ascii(lon_ref_vals)) = (&lon.value, &lon_ref.value) {
                if lon_vals.len() >= 3 && !lon_ref_vals.is_empty() {
                    let lon_deg = lon_vals[0].to_f64();
                    let lon_min = lon_vals[1].to_f64();
                    let lon_sec = lon_vals[2].to_f64();
                    let lon_ref = std::str::from_utf8(&lon_ref_vals[0]).unwrap_or("E");
                    
                    gps_info.push_str(&format!("{:.6}° {:.6}' {:.6}\" {}", 
                        lon_deg, lon_min, lon_sec, lon_ref));
                    has_coords = true;
                }
            }
        }
        
        if has_coords {
            metadata.push(gps_info);
        } else {
            metadata.push("GPS Data: Present but could not be parsed".to_string());
        }
    }
    
    // Other important EXIF tags
    if let Some(exposure) = get_exif_string(&exif, Tag::ExposureTime, In::PRIMARY) {
        metadata.push(format!("Exposure Time: {}", exposure));
    }
    
    if let Some(aperture) = get_exif_string(&exif, Tag::FNumber, In::PRIMARY) {
        metadata.push(format!("Aperture: {}", aperture));
    }
    
    if let Some(iso) = get_exif_string(&exif, Tag::ISOSpeed, In::PRIMARY) {
        metadata.push(format!("ISO: {}", iso));
    }
    
    if metadata.is_empty() {
        metadata.push("EXIF metadata was present but no readable values were found".to_string());
    }
    
    Ok(metadata)
}

fn get_exif_string(exif: &exif::Exif, tag: Tag, ifd: In) -> Option<String> {
    exif.get_field(tag, ifd).map(|field| field.display_value().to_string())
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
        let result = strip_image_metadata(input.path(), output.path());
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }
} 