use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use super::model::Model;
use super::widget;

pub fn view(model: &Model, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let main_area = chunks[0];
    let help_area = chunks[1];

    widget::calendar::render(f, main_area, model);
    widget::help_bar::render(f, help_area, model);

    if model.event_modal_open {
        widget::event_modal::render(f, model);
    }

    if model.delete_confirm.is_some() {
        widget::delete_confirm::render(f, model);
    }

    if model.event_form.is_some() {
        widget::event_form::render(f, model);
    }
}
