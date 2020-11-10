use crate::app::App;
use crate::app::Context::TopicListPage;
use crate::model::Event;
use crossterm::event::KeyCode;
use log::trace;

pub fn handle_key(event: Event<KeyCode>, app: &mut App) {
    match event {
        Event::Input(key) => match key {
            KeyCode::Esc => app.switch_context(TopicListPage),
            KeyCode::Char('o') => trace!("check one specific partition/offset"),
            KeyCode::Char('l') => trace!("live tail topic"),
            _ => {}
        },
        Event::Tick => app.load_topic_detail(),
    }
}
