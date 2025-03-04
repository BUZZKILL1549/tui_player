use std::error::Error;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, List, ListItem, ListState, Wrap},
};
mod music_manipulation;
use music_manipulation::*;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    
    let music_dir = "/home/buzzkill/Music/";
    let music_files = get_music(music_dir);
    let music_list: Vec<String> = convert_to_string(&music_files);
    
    // Initialize list state for scrolling
    let mut list_state = ListState::default()
        .with_selected(Some(0)); // Optional: start with first item selected
    
    let music_info = "Now Playing:\nBohemian Rhapsody\nby Queen\n\nAlbum: A Night at the Opera\nDuration: 5:55";
    
    // Main loop
    loop {
        // Draw the UI
        terminal.draw(|frame| ui(frame, &music_list, &mut list_state, music_info))?;
        
        // Handle events
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down => {
                    // Move selection down
                    let i = match list_state.selected() {
                        Some(i) => (i + 1) % music_list.len(),
                        None => 0,
                    };
                    list_state.select(Some(i));
                }
                KeyCode::Up => {
                    // Move selection up
                    let i = match list_state.selected() {
                        Some(i) => (i + music_list.len() - 1) % music_list.len(),
                        None => 0,
                    };
                    list_state.select(Some(i));
                }
                _ => {}
            }
        }
    }
    
    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().clear()?;
    terminal.backend_mut().flush()?;
    Ok(())
}

fn ui(frame: &mut Frame, music_list: &[String], list_state: &mut ListState, music_info: &str) {
    // Create the main layout
    let vertical = Layout::vertical([
        Constraint::Length(1),   // Title bar
        Constraint::Min(0)       // Main content
    ]);
    let [title_area, main_area] = vertical.areas(frame.area());
    
    // Create horizontal layout for music list and info
    let horizontal = Layout::horizontal([
        Constraint::Percentage(40),  // Music List
        Constraint::Percentage(60)   // Music Info
    ]);
    let [music_list_area, music_info_area] = horizontal.areas(main_area);
    
    // Title widget
    let title = Paragraph::new("Rust Music Player")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, title_area);
    
    // Music List widget with scrolling
    let music_list_block = Block::default()
        .title("Music List")
        .borders(Borders::ALL);
    
    // Convert music list to ListItems
    let items: Vec<ListItem> = music_list
        .iter()
        .map(|song| ListItem::new(song.as_str()))
        .collect();
    
    let list = List::new(items)
        .block(music_list_block)
        .highlight_style(Style::default().fg(Color::Yellow));
    
    // Render the list with state for scrolling
    frame.render_stateful_widget(list, music_list_area, list_state);
    
    // Music Info widget
    let music_info_block = Block::default()
        .title("Music Info")
        .borders(Borders::ALL);
    let music_info_text = Paragraph::new(music_info)
        .block(music_info_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(music_info_text, music_info_area);
}
