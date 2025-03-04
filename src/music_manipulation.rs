use std::path::PathBuf;
use walkdir::WalkDir;

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

