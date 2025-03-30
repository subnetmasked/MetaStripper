use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::fs;
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
    
    /// Recursively process subdirectories
    #[arg(short = 'r', long)]
    recursive: bool,
    
    /// Preview what would be processed without making changes
    #[arg(long)]
    dry_run: bool,
    
    /// Create backup of original files (.bak extension)
    #[arg(short = 'b', long)]
    backup: bool,
    
    /// Process only image files
    #[arg(long)]
    only_images: bool,
    
    /// Process only video files
    #[arg(long)]
    only_videos: bool,
    
    /// Process only PDF files
    #[arg(long)]
    only_pdfs: bool,
    
    /// Show statistics summary
    #[arg(short = 's', long)]
    stats: bool,
    
    /// Suppress all output except errors
    #[arg(short = 'q', long)]
    quiet: bool,
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    file_type: FileType,
}

#[derive(Debug, PartialEq)]
enum FileType {
    Image,
    Video,
    PDF,
    Unknown,
}

#[derive(Debug, Default)]
struct ProcessingStats {
    files_processed: usize,
    files_skipped: usize,
    files_failed: usize,
    metadata_items_removed: usize,
    by_type: std::collections::HashMap<String, usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    if args.verbose && !args.quiet {
        env_logger::init();
    }

    // Don't allow contradictory options
    if args.quiet && (args.verbose || args.show_metadata) {
        eprintln!("Warning: --quiet mode enabled, --verbose and --show-metadata will be ignored");
    }
    
    // Validate that only one file type filter is used
    let file_filters = [args.only_images, args.only_videos, args.only_pdfs].iter().filter(|&&f| f).count();
    if file_filters > 1 {
        anyhow::bail!("Only one file type filter can be used at a time");
    }

    // Validate output directory if specified
    if let Some(ref output_dir) = args.output_dir {
        if !output_dir.exists() && !args.dry_run {
            std::fs::create_dir_all(output_dir)
                .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;
        }
    }

    // Collect all files to process
    let files: Vec<FileInfo> = args.inputs
        .iter()
        .flat_map(|input| {
            if input.is_dir() {
                let walker = if args.recursive {
                    WalkDir::new(input)
                } else {
                    WalkDir::new(input).max_depth(1)
                };
                
                walker
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                    .map(|e| {
                        let file_type = determine_file_type(e.path());
                        FileInfo {
                            path: e.path().to_path_buf(),
                            file_type,
                        }
                    })
                    .filter(|file| should_process_file_type(&file.file_type, &args))
                    .collect()
            } else {
                let file_type = determine_file_type(input);
                if should_process_file_type(&file_type, &args) {
                    vec![FileInfo {
                        path: input.clone(),
                        file_type,
                    }]
                } else {
                    vec![]
                }
            }
        })
        .collect();

    if files.is_empty() {
        anyhow::bail!("No valid files found to process");
    }

    if args.dry_run && !args.quiet {
        println!("DRY RUN - No files will be modified");
        println!("\nFiles that would be processed:");
        for file in &files {
            println!("  {} ({})", file.path.display(), file_type_to_string(&file.file_type));
        }
        println!("\nTotal: {} files", files.len());
        return Ok(());
    }

    // Create progress bar unless in quiet mode
    let pb = if !args.quiet {
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut stats = ProcessingStats::default();

    // Process files in parallel
    let results: Vec<_> = files.par_iter()
        .map(|file| {
            // Create backup if requested
            if args.backup && !args.dry_run {
                let backup_path = format!("{}.bak", file.path.display());
                if let Err(e) = fs::copy(&file.path, &backup_path) {
                    warn!("Failed to create backup for {}: {}", file.path.display(), e);
                }
            }
            
            let result = process_file(file, &args);
            
            if let Some(pb) = &pb {
                pb.inc(1);
            }
            
            (file, result)
        })
        .collect();
    
    if let Some(pb) = pb {
        pb.finish_with_message("Processing complete");
    }
    
    // Collect statistics
    for (file, result) in &results {
        match file.file_type {
            FileType::Image => *stats.by_type.entry("Images".to_string()).or_insert(0) += 1,
            FileType::Video => *stats.by_type.entry("Videos".to_string()).or_insert(0) += 1,
            FileType::PDF => *stats.by_type.entry("PDFs".to_string()).or_insert(0) += 1,
            FileType::Unknown => *stats.by_type.entry("Unknown".to_string()).or_insert(0) += 1,
        }
        
        match result {
            Ok(metadata) => {
                stats.files_processed += 1;
                stats.metadata_items_removed += metadata.len();
            }
            Err(_) => {
                stats.files_failed += 1;
            }
        }
    }
    
    // Display results after the progress bar is done
    if args.show_metadata && !args.quiet {
        println!("\nRemoved metadata report:");
        for (file, result) in &results {
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
    
    // Display statistics if requested
    if args.stats && !args.quiet {
        println!("\nProcessing Statistics:");
        println!("  Files processed successfully: {}", stats.files_processed);
        println!("  Files failed: {}", stats.files_failed);
        println!("  Total metadata items removed: {}", stats.metadata_items_removed);
        println!("\n  By File Type:");
        for (file_type, count) in stats.by_type {
            println!("    {}: {}", file_type, count);
        }
    }
    
    Ok(())
}

fn file_type_to_string(file_type: &FileType) -> &'static str {
    match file_type {
        FileType::Image => "Image",
        FileType::Video => "Video",
        FileType::PDF => "PDF",
        FileType::Unknown => "Unknown",
    }
}

fn should_process_file_type(file_type: &FileType, args: &Args) -> bool {
    if args.only_images {
        return *file_type == FileType::Image;
    } else if args.only_videos {
        return *file_type == FileType::Video;
    } else if args.only_pdfs {
        return *file_type == FileType::PDF;
    }
    // Process all supported types by default
    *file_type != FileType::Unknown
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

    // Skip actual processing in dry run mode
    if args.dry_run {
        return Ok(vec!["Dry run - no metadata removed".to_string()]);
    }

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
        if args.verbose && !args.quiet {
            info!("Successfully processed: {}", file.path.display());
            if !metadata.is_empty() {
                info!("Removed {} metadata items", metadata.len());
            }
        }
    }
    
    result
}
