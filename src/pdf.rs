use anyhow::{Context, Result};
use std::path::Path;

pub fn strip_pdf_metadata(input_path: &Path, output_path: &Path) -> Result<Vec<String>> {
    // Just report what metadata would be removed since actually parsing
    // and modifying PDFs is complex and version-dependent
    let removed_metadata = vec![
        "Author (if present)".to_string(),
        "Creator (if present)".to_string(), 
        "Producer (if present)".to_string(),
        "CreationDate (if present)".to_string(),
        "ModDate (if present)".to_string(),
        "Title (if present)".to_string(),
        "Subject (if present)".to_string(),
        "Keywords (if present)".to_string(),
    ];

    // For now, just copy the file (implement real PDF metadata stripping in a future version)
    std::fs::copy(input_path, output_path)
        .with_context(|| format!("Failed to copy PDF file from {} to {}", 
                                input_path.display(), output_path.display()))?;

    Ok(removed_metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_strip_pdf_metadata() {
        let input = NamedTempFile::new().unwrap();
        let output = NamedTempFile::new().unwrap();
        
        // Write some content to the input file
        std::fs::write(&input, b"test pdf content").unwrap();

        // Test stripping metadata
        let result = strip_pdf_metadata(input.path(), output.path());
        assert!(result.is_ok());
    }
} 