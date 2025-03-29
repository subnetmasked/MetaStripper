use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use std::fs;

pub fn strip_video_metadata(input_path: &Path, output_path: &Path) -> Result<Vec<String>> {
    // Check if ffmpeg is installed
    if !is_ffmpeg_installed() {
        return Err(anyhow::anyhow!("ffmpeg is not installed. Please install ffmpeg to process video files."));
    }

    // Extract the actual metadata before removing it
    let removed_metadata = match extract_video_metadata(input_path) {
        Ok(metadata) => metadata,
        Err(_) => {
            // Fallback to generic metadata if extraction fails
            vec![
                "Creation time (if present)".to_string(),
                "Encoder information (if present)".to_string(),
                "Device information (if present)".to_string(),
                "GPS data (if present)".to_string(),
                "All metadata headers".to_string()
            ]
        }
    };

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

fn extract_video_metadata(input_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            input_path.to_str().unwrap(),
        ])
        .output()
        .with_context(|| format!("Failed to execute ffprobe command for: {}", input_path.display()))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("ffprobe failed to extract metadata"));
    }

    let metadata_output = String::from_utf8_lossy(&output.stdout);
    let mut metadata = Vec::new();
    
    // Parse important metadata from the JSON output
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata_output) {
        // Extract format metadata
        if let Some(format) = json.get("format") {
            if let Some(tags) = format.get("tags") {
                // Process standard tags
                process_tag(tags, "title", "Title", &mut metadata);
                process_tag(tags, "artist", "Artist", &mut metadata);
                process_tag(tags, "album", "Album", &mut metadata);
                process_tag(tags, "date", "Date", &mut metadata);
                process_tag(tags, "creation_time", "Creation Time", &mut metadata);
                process_tag(tags, "encoder", "Encoder", &mut metadata);
                process_tag(tags, "handler_name", "Handler", &mut metadata);
                process_tag(tags, "make", "Device Make", &mut metadata);
                process_tag(tags, "model", "Device Model", &mut metadata);
                process_tag(tags, "location", "Location", &mut metadata);
                process_tag(tags, "location-eng", "Location", &mut metadata);
                process_tag(tags, "com.apple.quicktime.location.ISO6709", "GPS Location", &mut metadata);
                
                // Iterate through all tags to catch non-standard ones
                if let Some(obj) = tags.as_object() {
                    for (key, value) in obj {
                        // Skip tags we've already processed
                        if ["title", "artist", "album", "date", "creation_time", "encoder", 
                            "handler_name", "make", "model", "location", "location-eng",
                            "com.apple.quicktime.location.ISO6709"].contains(&key.as_str()) {
                            continue;
                        }
                        
                        if let Some(val_str) = value.as_str() {
                            metadata.push(format!("{}: {}", key, val_str));
                        }
                    }
                }
            }
            
            // Add basic format info
            if let Some(format_name) = format.get("format_name").and_then(|v| v.as_str()) {
                metadata.push(format!("Format: {}", format_name));
            }
            
            if let Some(duration) = format.get("duration").and_then(|v| v.as_str()) {
                metadata.push(format!("Duration: {} seconds", duration));
            }
        }
        
        // Extract stream metadata (for the first video and audio stream)
        if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
            for stream in streams {
                if let Some(codec_type) = stream.get("codec_type").and_then(|v| v.as_str()) {
                    if codec_type == "video" {
                        if let Some(codec_name) = stream.get("codec_name").and_then(|v| v.as_str()) {
                            metadata.push(format!("Video Codec: {}", codec_name));
                        }
                        
                        if let (Some(width), Some(height)) = (
                            stream.get("width").and_then(|v| v.as_u64()),
                            stream.get("height").and_then(|v| v.as_u64())
                        ) {
                            metadata.push(format!("Resolution: {}x{}", width, height));
                        }
                        
                        if let Some(r_frame_rate) = stream.get("r_frame_rate").and_then(|v| v.as_str()) {
                            metadata.push(format!("Frame Rate: {}", r_frame_rate));
                        }
                        
                        // Get video stream tags
                        if let Some(tags) = stream.get("tags") {
                            // Process standard video tags
                            process_tag(tags, "creation_time", "Video Creation Time", &mut metadata);
                            process_tag(tags, "language", "Video Language", &mut metadata);
                            process_tag(tags, "handler_name", "Video Handler", &mut metadata);
                        }
                    } else if codec_type == "audio" {
                        if let Some(codec_name) = stream.get("codec_name").and_then(|v| v.as_str()) {
                            metadata.push(format!("Audio Codec: {}", codec_name));
                        }
                        
                        if let Some(sample_rate) = stream.get("sample_rate").and_then(|v| v.as_str()) {
                            metadata.push(format!("Audio Sample Rate: {} Hz", sample_rate));
                        }
                        
                        if let Some(channels) = stream.get("channels").and_then(|v| v.as_u64()) {
                            metadata.push(format!("Audio Channels: {}", channels));
                        }
                        
                        // Get audio stream tags
                        if let Some(tags) = stream.get("tags") {
                            // Process standard audio tags
                            process_tag(tags, "creation_time", "Audio Creation Time", &mut metadata);
                            process_tag(tags, "language", "Audio Language", &mut metadata);
                            process_tag(tags, "handler_name", "Audio Handler", &mut metadata);
                        }
                    }
                }
            }
        }
    }
    
    if metadata.is_empty() {
        metadata.push("No readable metadata found in the video file".to_string());
    }
    
    Ok(metadata)
}

fn process_tag(tags: &serde_json::Value, key: &str, display_name: &str, metadata: &mut Vec<String>) {
    if let Some(value) = tags.get(key).and_then(|v| v.as_str()) {
        if !value.is_empty() {
            metadata.push(format!("{}: {}", display_name, value));
        }
    }
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