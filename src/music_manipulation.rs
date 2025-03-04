use std::path::PathBuf;
use walkdir::WalkDir;

pub fn get_music(directory: &str) -> Vec<PathBuf> {
    // Define common music file extensions
    let music_extensions = [
        "mp3", "flac", "wav", "aac", 
        "ogg", "m4a", "wma", "alac"
    ];

    // Scan directory recursively and collect music files
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            // Check if it's a file (not a directory)
            entry.file_type().is_file()
        })
        .filter_map(|entry| {
            // Get the path of the file
            let path = entry.path().to_path_buf();
            
            // Check if the file extension matches music extensions
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
