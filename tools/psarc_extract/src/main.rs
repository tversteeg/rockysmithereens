use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use psarc::PlaystationArchive;
use rodio_wem::WemDecoder;

/// Command line arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, propagate_version = true)]
struct Cli {
    /// Path to a Rocksmith '*.psarc' file.
    #[clap(value_parser)]
    path: PathBuf,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List all paths in the psarc file.
    List,
    /// Export a specific file to the target destination.
    Extract {
        /// Which file to export.
        #[clap(value_parser)]
        path: String,
        /// Target destination of the file.
        #[clap(value_parser)]
        target: PathBuf,
    },
    /// Convert a music file to an ogg/vorbis file.
    ConvertOgg {
        /// Which file to export.
        #[clap(value_parser)]
        path: String,
        /// Target destination of the file.
        #[clap(value_parser)]
        target: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Open the archive
    let mut file = File::open(cli.path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let archive = PlaystationArchive::parse(&buf)?;

    match cli.command {
        Commands::List => archive.paths_iter().for_each(|file| println!("{}", file)),
        Commands::Extract { path, target } => {
            let path_index = archive.index_for_path(&path).expect("path not in archive");
            let extracted = archive.read_file(path_index)?;

            let mut target_file = File::create(&target)?;
            target_file.write_all(&extracted)?;

            println!("written to {:?}", target);
        }
        Commands::ConvertOgg { path, target } => {
            let path_index = archive.index_for_path(&path).expect("path not in archive");
            let extracted = archive.read_file(path_index)?;

            let mut target_file = File::create(&target)?;

            log::info!("parsing as vorbis");

            let decoder = WemDecoder::new(&extracted)?;

            target_file.write_all(&extracted)?;

            println!("written to {:?}", target);
        }
    }

    Ok(())
}
