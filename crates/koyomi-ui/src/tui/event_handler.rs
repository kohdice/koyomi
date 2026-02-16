use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::message::Message;
use super::model::{Focus, Model};

pub fn handle_key_event(model: &Model, key: KeyEvent) -> Option<Message> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    if model.detail_modal_open {
        return handle_detail_modal_key(key);
    }

    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        _ => match model.focus {
            Focus::Calendar => handle_calendar_key(model, key),
            Focus::EventList => handle_event_list_key(key),
        },
    }
}

fn handle_detail_modal_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::DetailScrollDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::DetailScrollUp),
        KeyCode::Char('g') => Some(Message::DetailScrollTop),
        KeyCode::Char('G') => Some(Message::DetailScrollBottom),
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseDetail),
        _ => None,
    }
}

fn handle_calendar_key(model: &Model, key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('h') | KeyCode::Left => Some(Message::MoveLeft),
        KeyCode::Char('j') | KeyCode::Down => Some(Message::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::MoveUp),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::MoveRight),
        KeyCode::Char('n') => Some(Message::NextMonth),
        KeyCode::Char('p') => Some(Message::PrevMonth),
        KeyCode::Char('t') => Some(Message::GoToToday),
        KeyCode::Char('e') => Some(Message::ToggleSidebar),
        KeyCode::Enter => {
            if model.sidebar_visible {
                Some(Message::ToggleFocus)
            } else {
                Some(Message::ToggleSidebar)
            }
        }
        KeyCode::Tab => {
            if model.sidebar_visible {
                Some(Message::ToggleFocus)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn handle_event_list_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::EventListDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::EventListUp),
        KeyCode::Enter => Some(Message::OpenDetail),
        KeyCode::Char('e') => Some(Message::ToggleSidebar),
        KeyCode::Tab | KeyCode::Esc => Some(Message::ToggleFocus),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn default_model() -> Model {
        Model::new("primary".to_string())
    }

    #[test]
    fn ctrl_c_always_quits() {
        let model = default_model();
        let msg =
            handle_key_event(&model, make_key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(matches!(msg, Some(Message::Quit)));
    }

    #[test]
    fn q_quits_in_calendar_mode() {
        let model = default_model();
        let msg = handle_key_event(&model, make_key(KeyCode::Char('q')));
        assert!(matches!(msg, Some(Message::Quit)));
    }

    #[test]
    fn hjkl_navigates_calendar() {
        let model = default_model();

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('h'))),
            Some(Message::MoveLeft)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('j'))),
            Some(Message::MoveDown)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('k'))),
            Some(Message::MoveUp)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('l'))),
            Some(Message::MoveRight)
        ));
    }

    #[test]
    fn n_p_t_navigates_months() {
        let model = default_model();

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('n'))),
            Some(Message::NextMonth)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('p'))),
            Some(Message::PrevMonth)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('t'))),
            Some(Message::GoToToday)
        ));
    }

    #[test]
    fn tab_does_nothing_when_sidebar_hidden() {
        let model = default_model();
        let msg = handle_key_event(&model, make_key(KeyCode::Tab));
        assert!(msg.is_none());
    }

    #[test]
    fn tab_toggles_focus_when_sidebar_visible() {
        let mut model = default_model();
        model.sidebar_visible = true;
        let msg = handle_key_event(&model, make_key(KeyCode::Tab));
        assert!(matches!(msg, Some(Message::ToggleFocus)));
    }

    #[test]
    fn event_list_jk_navigates() {
        let mut model = default_model();
        model.focus = Focus::EventList;

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('j'))),
            Some(Message::EventListDown)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('k'))),
            Some(Message::EventListUp)
        ));
    }

    #[test]
    fn event_list_enter_opens_detail() {
        let mut model = default_model();
        model.focus = Focus::EventList;
        let msg = handle_key_event(&model, make_key(KeyCode::Enter));
        assert!(matches!(msg, Some(Message::OpenDetail)));
    }

    #[test]
    fn detail_modal_scroll_keys() {
        let mut model = default_model();
        model.detail_modal_open = true;

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('j'))),
            Some(Message::DetailScrollDown)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('k'))),
            Some(Message::DetailScrollUp)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('g'))),
            Some(Message::DetailScrollTop)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('G'))),
            Some(Message::DetailScrollBottom)
        ));
    }

    #[test]
    fn detail_modal_close_keys() {
        let mut model = default_model();
        model.detail_modal_open = true;

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Enter)),
            Some(Message::CloseDetail)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Esc)),
            Some(Message::CloseDetail)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('q'))),
            Some(Message::CloseDetail)
        ));
    }
}
