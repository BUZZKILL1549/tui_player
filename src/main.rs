use crossterm::event::{self, Event};
use ratatui::{
    layout::{Constraint, Layout},
    widgets::Block,
    Frame,
};

fn main() {
    let mut terminal = ratatui::init();
    loop {
        terminal.draw(draw).expect("Failed to draw frame");
        if matches!(event::read().expect("Failed to read event"), Event::Key(_)) {
            break;
        }
    }
    ratatui::restore();
}


fn draw(frame: &mut Frame) {
    use Constraint::{Fill, Length, Min};

    let vertical = Layout::vertical([Length(1), Min(0)]);
    let [title_area, main_area] = vertical.areas(frame.area());
    let horizontal = Layout::horizontal([Fill(1); 2]);
    let [left_area, right_area] = horizontal.areas(main_area);

    frame.render_widget(Block::bordered().title("Music Player"), title_area);
    frame.render_widget(Block::bordered().title("Music List"), left_area);
    frame.render_widget(Block::bordered().title("Music Info"), right_area);
}
