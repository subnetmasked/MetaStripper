use anyhow::{Context, Result};
use pdf::file::FileOptions;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub fn strip_pdf_metadata(input_path: &Path, output_path: &Path) -> Result<()> {
    // Open the PDF file
    let file = File::open(input_path)
        .with_context(|| format!("Failed to open PDF: {}", input_path.display()))?;
    
    let reader = BufReader::new(file);
    let mut pdf = pdf::file::File::from_reader_uncached(reader)
        .with_context(|| format!("Failed to parse PDF: {}", input_path.display()))?;

    // Remove metadata from the document info dictionary
    if let Some(mut info) = pdf.trailer.info_dict() {
        info.remove("Author");
        info.remove("Creator");
        info.remove("Producer");
        info.remove("CreationDate");
        info.remove("ModDate");
        info.remove("Title");
        info.remove("Subject");
        info.remove("Keywords");
    }

    // Write the modified PDF to the output file
    let output_file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
    
    let mut writer = BufWriter::new(output_file);
    pdf.save_to(&mut writer)
        .with_context(|| format!("Failed to save PDF: {}", output_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_strip_pdf_metadata() {
        let input = NamedTempFile::new().unwrap();
        let output = NamedTempFile::new().unwrap();

        // Create a simple test PDF
        let mut pdf = pdf::file::File::default();
        pdf.save_to(&mut File::create(&input).unwrap()).unwrap();

        // Test stripping metadata
        assert!(strip_pdf_metadata(input.path(), output.path()).is_ok());
    }
} 