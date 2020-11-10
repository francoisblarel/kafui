use crate::app::App;
use crate::model::Event;
use crossterm::event::KeyCode;

pub fn handle_key(event: Event<KeyCode>, app: &mut App) {
    match event {
        Event::Input(key) => match key {
            KeyCode::Char('s') => app.change_message(String::from("toto")),
            KeyCode::Up => app.select_previous_topic(),
            KeyCode::Down => app.select_next_topic(),
            KeyCode::Enter => app.select_current_topic(),
            _ => {}
        },
        Event::Tick => app.change_message(format!("A new tick arrive {}", app.message)),
    }
}
