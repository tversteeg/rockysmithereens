use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use rockysmithereens_parser::SongFile;
use rodio::{Decoder, OutputStream, Source};

/// Command line arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, propagate_version = true)]
struct Cli {
    /// Path to a Rocksmith '*.psarc' file.
    #[clap(value_parser)]
    path: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Open the archive
    let mut file = File::open(cli.path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    // Read the archive
    let song = SongFile::parse(&buf)?;

    // Find song information
    let attributes = song.manifests[0].attributes();
    println!(
        "playing song '{}' by '{}' from album '{}' for '{}' seconds",
        attributes.song_name, attributes.artist_name, attributes.album_name, attributes.song_length
    );

    // Convert the raw song binary to an audio source
    let file = Cursor::new(song.ogg(0)?);
    let decoder = rodio_wem::vorbis_from_wem(file)?;

    // Play the song
    let (_stream, stream_handle) = OutputStream::try_default()?;
    stream_handle.play_raw(decoder.convert_samples())?;

    // Sleep for the duration of the song
    std::thread::sleep(Duration::from_secs((attributes.song_length + 1.0) as u64));

    println!("song ended");

    Ok(())
}
