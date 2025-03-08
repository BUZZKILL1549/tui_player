use std::error::Error;
use std::io;
use std::path::PathBuf;
use crossterm::{
    execute,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem, Gauge},
};

mod music_manipulation;
use music_manipulation::*;

mod app;
use app::*;

mod playback;
use playback::*;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let music_dir = "/home/buzzkill/Music/";
    let music_files = get_music(music_dir);
    
    let mut app = App::new(&music_files);
    let mut player = AudioPlayer::new();

    let music_files_full_path: Vec<PathBuf> = music_files;
    
    loop {
        player.update_position();

        let current_song_tags = app.current_song_tags.clone();

        terminal.draw(|frame| ui(frame, &mut app, &current_song_tags, &player))?;
        
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    AppMode::Normal => match (key.code, key.modifiers) {
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            if let Some(selected_filename) = app.get_selected_song() {
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
                                            app.current_song_tags = tags.iter()
                                                .map(|(key, value)| format!("{}: {}", key, value))
                                                .collect::<Vec<String>>()
                                                .join("\n");
                                        },
                                        Err(_) => {
                                            app.current_song_tags = "Unable to read tags".to_string();
                                        }
                                    }
                                }
                                let full_path_cloned = full_path.cloned();
                                player.play_song(full_path_cloned);
                            }
                        }
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
    }
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    player.stop();
    Ok(())
}

 fn ui(frame: &mut Frame, app: &mut App, music_info: &str, player: &AudioPlayer) {
    let main_layout = match app.mode {
        AppMode::Search => {
            Layout::vertical([
                Constraint::Length(3),   // Top bar 
                Constraint::Length(3),   // Search box
                Constraint::Min(0),      // Main content
                Constraint::Length(3)    // Progress bar
            ]).areas::<4>(frame.area()).to_vec()
        },
        AppMode::Normal => {
            Layout::vertical([
                Constraint::Length(3),   // Top bar 
                Constraint::Min(0),      // Main content
                Constraint::Length(3)    // Progress bar
            ]).areas::<3>(frame.area()).to_vec()
        }
    };
    
    let top_bar_block = Block::default()
        .borders(Borders::ALL);
    
    let top_inner_area = top_bar_block.inner(main_layout[0]);
    
    frame.render_widget(top_bar_block, main_layout[0]);
    
    let top_areas = Layout::horizontal([
        Constraint::Percentage(70),  // Title 
        Constraint::Percentage(30)   // Mode 
    ]).areas::<2>(top_inner_area);
    
    let title = Paragraph::new("Music Player");
    frame.render_widget(title, top_areas[0]);
    
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL".to_string(),
        AppMode::Search => "SEARCH".to_string(),
    };
    let mode_widget = Paragraph::new(mode_text)
        .alignment(Alignment::Right); 
    frame.render_widget(mode_widget, top_areas[1]);
    
    let progress_index = if app.mode == AppMode::Search { 3 } else { 2 };
    
    if player.is_playing() {
        let progress_percentage = (player.get_progress() * 100.0) as u16;
        let time_text = player.format_time();
        let song_name = player.current_song_name().unwrap_or_else(|| "Unknown".to_string());
        
        let progress_block = Block::default()
            .title(format!("Now Playing: {} - {}", song_name, time_text))
            .borders(Borders::ALL);
        
        let gauge = Gauge::default()
            .block(progress_block)
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(player.get_progress().into())
            .label(format!("{}%", progress_percentage));
        
        frame.render_widget(gauge, main_layout[progress_index]);
    } else {
        let progress_block = Block::default()
            .title("Not Playing")
            .borders(Borders::ALL);
        
        frame.render_widget(progress_block, main_layout[progress_index]);
    }
    
    match app.mode {
        AppMode::Search => {
            let search_text = format!("Search: {}", app.search_input);
            let search_box = Paragraph::new(search_text)
                .block(Block::default()
                    .title("Search")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)))
                .style(Style::default().fg(Color::White).bg(Color::Black));
            frame.render_widget(search_box, main_layout[1]);
            
            let horizontal = Layout::horizontal([
                Constraint::Percentage(60),
                Constraint::Percentage(40)
            ]);
            let [music_list_area, music_info_area] = horizontal.areas(main_layout[2]);
            
            render_music_content(frame, app, music_info, music_list_area, music_info_area);
        },
        AppMode::Normal => {
            let horizontal = Layout::horizontal([
                Constraint::Percentage(60),
                Constraint::Percentage(40)
            ]);
            let [music_list_area, music_info_area] = horizontal.areas(main_layout[1]);
            
            render_music_content(frame, app, music_info, music_list_area, music_info_area);
        }
    }
}

fn render_music_content(
    frame: &mut Frame,
    app: &mut App,
    music_info: &str,
    music_list_area: Rect,
    music_info_area: Rect
) {
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
