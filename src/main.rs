use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod image;
mod pdf;

// Import the module but not directly the function to avoid linker errors
mod video;

use image::strip_image_metadata;
use pdf::strip_pdf_metadata;

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
    
    /// Show detailed report of metadata removed from each file
    #[arg(short = 'm', long)]
    show_metadata: bool,
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
    PDF,
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
    let results: Vec<_> = files.par_iter()
        .map(|file| {
            let result = process_file(file, &args);
            pb.inc(1);
            (file, result)
        })
        .collect();
    
    pb.finish_with_message("Processing complete");
    
    // Display results after the progress bar is done
    if args.show_metadata {
        println!("\nRemoved metadata report:");
        for (file, result) in results {
            match result {
                Ok(metadata) => {
                    if !metadata.is_empty() {
                        println!("\n{}: ", file.path.display());
                        for item in metadata {
                            println!("  - {}", item);
                        }
                    }
                }
                Err(e) => {
                    println!("\n{}: Failed - {}", file.path.display(), e);
                }
            }
        }
    }
    
    Ok(())
}

fn determine_file_type(path: &Path) -> FileType {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" => FileType::Image,
            "mp4" | "mov" | "avi" | "mkv" => FileType::Video,
            "pdf" => FileType::PDF,
            _ => FileType::Unknown,
        }
    } else {
        FileType::Unknown
    }
}

fn process_file(file: &FileInfo, args: &Args) -> Result<Vec<String>> {
    let output_path = if args.overwrite {
        file.path.clone()
    } else {
        let output_dir = args.output_dir.as_ref()
            .map(|d| d.clone())
            .unwrap_or_else(|| file.path.parent().unwrap().to_path_buf());
        
        let file_name = file.path.file_name().unwrap();
        output_dir.join(file_name)
    };

    let result = match file.file_type {
        FileType::Image => strip_image_metadata(&file.path, &output_path),
        FileType::Video => video::strip_video_metadata(&file.path, &output_path),
        FileType::PDF => strip_pdf_metadata(&file.path, &output_path),
        FileType::Unknown => {
            warn!("Unsupported file type: {}", file.path.display());
            Ok(vec!["Unsupported file type - no metadata removed".to_string()])
        }
    };
    
    if let Ok(ref metadata) = result {
        if args.verbose {
            info!("Successfully processed: {}", file.path.display());
            if !metadata.is_empty() {
                info!("Removed {} metadata items", metadata.len());
            }
        }
    }
    
    result
}
