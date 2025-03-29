use image::{ImageBuffer, Rgb};
use std::fs::File;
use std::io::BufWriter;

fn main() {
    // Create test directory if it doesn't exist
    std::fs::create_dir_all("test_files").unwrap();

    // Create a test image
    let img = ImageBuffer::from_fn(100, 100, |_, _| {
        Rgb([255u8, 255, 255])
    });
    img.save("test_files/test.jpg").unwrap();

    // Create a test video using ffmpeg
    if is_ffmpeg_installed() {
        std::process::Command::new("ffmpeg")
            .args([
                "-f", "lavfi",
                "-i", "testsrc=duration=1:size=1280x720:rate=30",
                "-metadata", "title=Test Video",
                "-metadata", "artist=Test Artist",
                "-c:v", "libx264",
                "test_files/test.mp4",
            ])
            .output()
            .unwrap();
    }

    println!("Test files created successfully!");
}

fn is_ffmpeg_installed() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
} 