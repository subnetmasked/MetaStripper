use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod image;
mod video;

use image::strip_image_metadata;
use video::strip_video_metadata;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input files or directories to process
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Overwrite original files instead of creating new ones
    #[arg(short = 'w', long)]
    overwrite: bool,

    /// Output directory for cleaned files (ignored if --overwrite is set)
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    file_type: FileType,
}

#[derive(Debug)]
enum FileType {
    Image,
    Video,
    Unknown,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    if args.verbose {
        env_logger::init();
    }

    // Validate output directory if specified
    if let Some(ref output_dir) = args.output_dir {
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;
        }
    }

    // Collect all files to process
    let files: Vec<FileInfo> = args.inputs
        .iter()
        .flat_map(|input| {
            if input.is_dir() {
                WalkDir::new(input)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| FileInfo {
                        path: e.path().to_path_buf(),
                        file_type: determine_file_type(e.path()),
                    })
                    .collect()
            } else {
                vec![FileInfo {
                    path: input.clone(),
                    file_type: determine_file_type(input),
                }]
            }
        })
        .collect();

    if files.is_empty() {
        anyhow::bail!("No valid files found to process");
    }

    // Create progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Process files in parallel
    files.par_iter().for_each(|file| {
        match process_file(file, &args) {
            Ok(_) => info!("Successfully processed: {}", file.path.display()),
            Err(e) => warn!("Failed to process {}: {}", file.path.display(), e),
        }
        pb.inc(1);
    });

    pb.finish_with_message("Processing complete");
    Ok(())
}

fn determine_file_type(path: &Path) -> FileType {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" => FileType::Image,
            "mp4" | "mov" | "avi" | "mkv" => FileType::Video,
            _ => FileType::Unknown,
        }
    } else {
        FileType::Unknown
    }
}

fn process_file(file: &FileInfo, args: &Args) -> Result<()> {
    let output_path = if args.overwrite {
        file.path.clone()
    } else {
        let output_dir = args.output_dir.as_ref()
            .map(|d| d.clone())
            .unwrap_or_else(|| file.path.parent().unwrap().to_path_buf());
        
        let file_name = file.path.file_name().unwrap();
        output_dir.join(file_name)
    };

    match file.file_type {
        FileType::Image => strip_image_metadata(&file.path, &output_path),
        FileType::Video => strip_video_metadata(&file.path, &output_path),
        FileType::Unknown => {
            warn!("Unsupported file type: {}", file.path.display());
            Ok(())
        }
    }
}
