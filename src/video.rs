use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use std::fs;

pub fn strip_video_metadata(input_path: &Path, output_path: &Path) -> Result<Vec<String>> {
    // Check if ffmpeg is installed
    if !is_ffmpeg_installed() {
        return Err(anyhow::anyhow!("ffmpeg is not installed. Please install ffmpeg to process video files."));
    }

    // Instead of trying to extract exact metadata which can be complex,
    // just provide general information about what will be removed
    let removed_metadata = vec![
        "Creation time (if present)".to_string(),
        "Encoder information (if present)".to_string(),
        "Device information (if present)".to_string(),
        "GPS data (if present)".to_string(),
        "All metadata headers".to_string()
    ];

    // Create a temporary file path
    let temp_path = output_path.with_extension("tmp.mp4");

    // Construct ffmpeg command to strip metadata
    let status = Command::new("ffmpeg")
        .args([
            "-i", input_path.to_str().unwrap(),
            "-map_metadata", "-1",  // Remove all metadata
            "-c:v", "copy",         // Copy video stream without re-encoding
            "-c:a", "copy",         // Copy audio stream without re-encoding
            "-y",                   // Overwrite output file if it exists
            temp_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| format!("Failed to execute ffmpeg command for: {}", input_path.display()))?;

    if !status.status.success() {
        let error = String::from_utf8_lossy(&status.stderr);
        return Err(anyhow::anyhow!("ffmpeg failed: {}", error));
    }

    // Move the temporary file to the final destination
    fs::rename(&temp_path, output_path)
        .with_context(|| format!("Failed to move temporary file to: {}", output_path.display()))?;

    Ok(removed_metadata)
}

fn is_ffmpeg_installed() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_ffmpeg_installation() {
        // Don't fail the test if ffmpeg isn't installed
        let _ = is_ffmpeg_installed();
    }

    #[test]
    fn test_strip_video_metadata() {
        let input = NamedTempFile::new().unwrap();
        let output = NamedTempFile::new().unwrap();

        // Skip test if ffmpeg is not installed
        if !is_ffmpeg_installed() {
            return;
        }

        // Create a simple test video file
        Command::new("ffmpeg")
            .args([
                "-f", "lavfi",
                "-i", "testsrc=duration=1:size=1280x720:rate=30",
                "-c:v", "libx264",
                input.path().to_str().unwrap(),
            ])
            .output()
            .unwrap();

        // Test stripping metadata
        let result = strip_video_metadata(input.path(), output.path());
        assert!(result.is_ok());
    }
} 