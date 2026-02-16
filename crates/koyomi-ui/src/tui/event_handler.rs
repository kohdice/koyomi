use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::message::Message;
use super::model::{Focus, Model};

pub fn handle_key_event(model: &Model, key: KeyEvent) -> Option<Message> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Message::Quit);
    }

    if model.event_modal_open {
        return handle_event_modal_key(model, key);
    }

    handle_calendar_key(key)
}

fn handle_calendar_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('h') | KeyCode::Left => Some(Message::MoveLeft),
        KeyCode::Char('j') | KeyCode::Down => Some(Message::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::MoveUp),
        KeyCode::Char('l') | KeyCode::Right => Some(Message::MoveRight),
        KeyCode::Char('n') => Some(Message::NextMonth),
        KeyCode::Char('p') => Some(Message::PrevMonth),
        KeyCode::Char('t') => Some(Message::GoToToday),
        KeyCode::Enter => Some(Message::OpenEventModal),
        _ => None,
    }
}

fn handle_event_modal_key(model: &Model, key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Message::CloseEventModal),
        KeyCode::Tab => Some(Message::ModalToggleFocus),
        _ => match model.focus {
            Focus::EventList => handle_modal_event_list_key(key),
            Focus::EventDetail => handle_modal_detail_key(key),
            Focus::Calendar => None,
        },
    }
}

fn handle_modal_event_list_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::EventListDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::EventListUp),
        _ => None,
    }
}

fn handle_modal_detail_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Message::DetailScrollDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Message::DetailScrollUp),
        KeyCode::Char('g') => Some(Message::DetailScrollTop),
        KeyCode::Char('G') => Some(Message::DetailScrollBottom),
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
        Model::new("primary".to_string(), koyomi_core::calendar::TimeZone::Jst)
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
    fn enter_opens_event_modal() {
        let model = default_model();
        let msg = handle_key_event(&model, make_key(KeyCode::Enter));
        assert!(matches!(msg, Some(Message::OpenEventModal)));
    }

    #[test]
    fn modal_tab_toggles_focus() {
        let mut model = default_model();
        model.event_modal_open = true;
        model.focus = Focus::EventList;
        let msg = handle_key_event(&model, make_key(KeyCode::Tab));
        assert!(matches!(msg, Some(Message::ModalToggleFocus)));
    }

    #[test]
    fn modal_esc_closes() {
        let mut model = default_model();
        model.event_modal_open = true;
        model.focus = Focus::EventList;

        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Esc)),
            Some(Message::CloseEventModal)
        ));
        assert!(matches!(
            handle_key_event(&model, make_key(KeyCode::Char('q'))),
            Some(Message::CloseEventModal)
        ));
    }

    #[test]
    fn modal_event_list_jk_navigates() {
        let mut model = default_model();
        model.event_modal_open = true;
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
    fn modal_detail_scroll_keys() {
        let mut model = default_model();
        model.event_modal_open = true;
        model.focus = Focus::EventDetail;

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
}
