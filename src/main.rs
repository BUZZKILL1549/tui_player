use std::error::Error;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    
    // Sample music data
    let music_list = vec![
        "1. Bohemian Rhapsody - Queen",
        "2. Stairway to Heaven - Led Zeppelin",
        "3. Imagine - John Lennon",
        "4. Smells Like Teen Spirit - Nirvana",
        "5. Hotel California - Eagles",
    ];

    let music_info = "Now Playing:\nBohemian Rhapsody\nby Queen\n\nAlbum: A Night at the Opera\nDuration: 5:55";

    // Main loop
    loop {
        // Draw the UI
        terminal.draw(|frame| ui(frame, &music_list, music_info))?;

        // Handle events
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
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

fn ui(frame: &mut Frame, music_list: &[&str], music_info: &str) {
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

    // Music List widget
    let music_list_block = Block::default()
        .title("Music List")
        .borders(Borders::ALL);
    let music_list_text = Paragraph::new(music_list.join("\n"))
        .block(music_list_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(music_list_text, music_list_area);

    // Music Info widget
    let music_info_block = Block::default()
        .title("Music Info")
        .borders(Borders::ALL);
    let music_info_text = Paragraph::new(music_info)
        .block(music_info_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(music_info_text, music_info_area);
}
