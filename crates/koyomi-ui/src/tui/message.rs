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

    ToggleSidebar,
    ToggleFocus,
    EventListUp,
    EventListDown,
    OpenDetail,
    CloseDetail,
    DetailScrollUp,
    DetailScrollDown,
    DetailScrollTop,
    DetailScrollBottom,

    RequestEvents { year: i32, month: u32 },
    EventsLoaded { year: i32, month: u32, calendar_name: String, events: Vec<Event> },
    EventsLoadFailed { error: String },

    Quit,
}
