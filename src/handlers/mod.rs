mod topic_detail;
mod topic_list;

use super::app::App;
use super::app::Context::{TopicDetailPage, TopicListPage};
use crate::model::Event;
use crossterm::event::KeyCode;
use std::borrow::BorrowMut;

pub fn handle_event(event: Event<KeyCode>, app: &mut App) {
    let context = app.context.borrow_mut();
    match context {
        TopicListPage => topic_list::handle_key(event, app),
        TopicDetailPage => topic_detail::handle_key(event, app),
    }
}
