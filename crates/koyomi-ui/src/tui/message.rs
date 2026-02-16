use std::collections::HashMap;

use koyomi_core::calendar::Event;

#[derive(Debug)]
pub enum Message {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    NextMonth,
    PrevMonth,
    GoToToday,

    OpenEventModal,
    CloseEventModal,
    ModalToggleFocus,
    EventListUp,
    EventListDown,
    DetailScrollUp,
    DetailScrollDown,
    DetailScrollTop,
    DetailScrollBottom,

    RequestEvents,
    EventsLoaded { calendar_name: String, events_by_month: HashMap<(i32, u32), Vec<Event>> },
    EventsLoadFailed { error: String },

    Quit,
}
