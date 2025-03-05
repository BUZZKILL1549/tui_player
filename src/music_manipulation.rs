use std::path::PathBuf;
use walkdir::WalkDir;
use lofty::{
    prelude::*,
    probe::Probe
};

pub fn get_music(directory: &str) -> Vec<PathBuf> {
    let music_extensions = [
        "mp3", "flac", "wav", "aac", 
        "ogg", "m4a", "wma", "alac"
    ];

    WalkDir::new(directory)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
        })
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| music_extensions.contains(&ext.to_lowercase().as_str()))
                .filter(|&is_music| is_music)
                .map(|_| path)
        })
        .collect()
}

pub fn convert_to_string(paths: &Vec<PathBuf>) -> Vec<String> {
    paths.iter()
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
        })
        .collect()
}

pub fn get_music_tags(path: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let tagged_file = Probe::open(path)
        .expect("ERROR: Bad path")
        .read()
        .expect("ERROR: Failed to read file");

    let tag = match tagged_file.primary_tag() {
        Some(primary_tag) => primary_tag,
        None => tagged_file.first_tag().expect("ERROR: No tags found"),
    };

    let mut tags = Vec::new();

    if let Some(title) = tag.title() {
        tags.push(("Title".to_string(), title.to_string()));
    }

    if let Some(artist) = tag.artist() {
        tags.push(("Artist".to_string(), artist.to_string()));
    } 

    if let Some(album) = tag.album() {
        tags.push(("Album".to_string(), album.to_string()));
    }

    if let Some(genre) = tag.genre() {
        tags.push(("Genre".to_string(), genre.to_string()));
    }

    Ok(tags)
} 
