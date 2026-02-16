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

    if model.sidebar_visible {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(main_area);

        widget::calendar::render(f, horizontal[0], model);
        widget::event_list::render(f, horizontal[1], model);
    } else {
        widget::calendar::render(f, main_area, model);
    }

    widget::help_bar::render(f, help_area, model);

    if model.detail_modal_open {
        widget::event_detail::render(f, model);
    }
}
