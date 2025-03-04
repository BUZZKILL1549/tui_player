use ratatui::widgets::ListState;
use std::path::PathBuf;

pub enum AppMode {
    Normal,
    Search,
}

pub struct App {
    pub music_list: Vec<String>,
    pub filtered_list: Vec<String>,
    pub list_state: ListState,
    pub mode: AppMode,
    pub search_input: String,
}

impl App {
    pub fn new(music_files: &[PathBuf]) -> Self {
        let music_list: Vec<String> = music_files
            .iter()
            .filter_map(|path| 
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            )
            .collect();

        Self {
            filtered_list: music_list.clone(),
            music_list,
            list_state: ListState::default().with_selected(Some(0)),
            mode: AppMode::Normal,
            search_input: String::new(),
        }
    }

    pub fn filter_list(&mut self) {
        if self.search_input.is_empty() {
            self.filtered_list = self.music_list.clone();
        } else {
            self.filtered_list = self.music_list
                .iter()
                .filter(|song| 
                    song.to_lowercase().contains(&self.search_input.to_lowercase())
                )
                .cloned()
                .collect();
        }
        self.list_state.select(Some(0));
    }

    pub fn move_down(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.filtered_list.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn move_up(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => (i + self.filtered_list.len() - 1) % self.filtered_list.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn half_page_down(&mut self, area_height: usize) {
        let half_page = area_height / 2;
        let current = self.list_state.selected().unwrap_or(0);
        let new_index = (current + half_page).min(self.filtered_list.len() - 1);
        self.list_state.select(Some(new_index));
    }

    pub fn half_page_up(&mut self, area_height: usize) {
        let half_page = area_height / 2;
        let current = self.list_state.selected().unwrap_or(0);
        let new_index = current.saturating_sub(half_page);
        self.list_state.select(Some(new_index));
    }

    pub fn get_selected_song(&self) -> Option<&String> {
        self.list_state
            .selected()
            .and_then(|index| self.filtered_list.get(index))
    }
}
