use std::error::Error;
use std::path::PathBuf;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem},
};

mod music_manipulation;
use music_manipulation::*;

mod app;
use app::*;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    
    let music_dir = "/home/buzzkill/Music/";
    let music_files = get_music(music_dir);
    
    let mut app = App::new(&music_files);

    let music_files_full_path: Vec<PathBuf> = music_files;
    
    loop {
        let music_info = if let Some(selected_filename) = app.get_selected_song() {
            let full_path = music_files_full_path
                .iter()
                .find(|path| 
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|s| s == selected_filename)
                );

            if let Some(path) = full_path {
                match get_music_tags(path.to_str().unwrap_or("")) {
                    Ok(tags) => {
                        tags.iter()
                            .map(|(key, value)| format!("{}: {}", key, value))
                            .collect::<Vec<String>>()
                            .join("\n")
                    },
                    Err(_) => "Unable to read tags".to_string()
                }
            } else {
                "No song selected".to_string()
            }
        } else {
            "No song selected".to_string()
        };

        terminal.draw(|frame| ui(frame, &mut app, &music_info))?;
        
        if let Event::Key(key) = event::read()? {
            match app.mode {
                AppMode::Normal => match (key.code, key.modifiers) {
                    (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
                        app.move_down();
                    }
                    (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
                        app.move_up();
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        let area_height = terminal.size()?.height as usize;
                        app.half_page_down(area_height);
                    }
                    (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                        let area_height = terminal.size()?.height as usize;
                        app.half_page_up(area_height);
                    }
                    (KeyCode::Char('g'), KeyModifiers::NONE) => {
                        app.list_state.select(Some(0));
                    }
                    (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                        app.list_state.select(Some(app.filtered_list.len() - 1));
                    }
                    (KeyCode::Char('/'), KeyModifiers::NONE) => {
                        app.mode = AppMode::Search;
                        app.search_input.clear();
                    }
                    (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, KeyModifiers::NONE) => break,
                    _ => {}
                },
                AppMode::Search => match key.code {
                    KeyCode::Char(c) => {
                        app.search_input.push(c);
                        app.filter_list();
                    }
                    KeyCode::Backspace => {
                        app.search_input.pop();
                        app.filter_list();
                    }
                    KeyCode::Esc => {
                        app.mode = AppMode::Normal;
                        app.search_input.clear();
                        app.filter_list();
                    }
                    KeyCode::Enter => {
                        app.mode = AppMode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }
    
    disable_raw_mode()?;
    terminal.backend_mut().clear()?;
    terminal.backend_mut().flush()?;
    Ok(())
}

fn ui(frame: &mut Frame, app: &mut App, music_info: &str) {
    let vertical = Layout::vertical([
        Constraint::Length(1),   // Title bar
        Constraint::Length(1),   // Search/Mode info
        Constraint::Min(0)       // Main content
    ]);
    let [title_area, mode_area, main_area] = vertical.areas(frame.area());
    
    let horizontal = Layout::horizontal([
        Constraint::Percentage(40),  // Music List
        Constraint::Percentage(60)   // Music Info
    ]);
    let [music_list_area, music_info_area] = horizontal.areas(main_area);
    
    let title = Paragraph::new("Rust Music Player")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, title_area);
    
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL".to_string(),
        AppMode::Search => format!("SEARCH: {}", app.search_input),
    };
    let mode_widget = Paragraph::new(mode_text)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(mode_widget, mode_area);
    
    let music_list_block = Block::default()
        .title("Music List")
        .borders(Borders::ALL);
    
    let items: Vec<ListItem> = app.filtered_list
        .iter()
        .map(|song| ListItem::new(song.as_str()))
        .collect();
    
    let list = List::new(items)
        .block(music_list_block)
        .highlight_style(Style::default().fg(Color::Yellow));
    
    frame.render_stateful_widget(list, music_list_area, &mut app.list_state);
    
    let music_info_block = Block::default()
        .title("Music Info")
        .borders(Borders::ALL);
    let music_info_text = Paragraph::new(music_info)
        .block(music_info_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(music_info_text, music_info_area);
}
