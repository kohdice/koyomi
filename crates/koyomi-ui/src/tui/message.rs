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
    RequestEvents,
    RefreshEvents,
    EventsLoaded { calendar_name: String, events_by_month: HashMap<(i32, u32), Vec<Event>> },
    EventsLoadFailed { error: String },

    // Delete
    OpenDeleteConfirm,
    ConfirmDelete,
    CancelDelete,
    DeleteSuccess,
    DeleteFailed { error: String },

    // Form (Add / Edit)
    OpenAddForm,
    OpenEditForm,
    FormInput { ch: char },
    FormBackspace,
    FormDelete,
    FormCursorLeft,
    FormCursorRight,
    FormCursorHome,
    FormCursorEnd,
    FormNextField,
    FormPrevField,
    FormReminderNext,
    FormReminderPrev,
    FormSubmit,
    FormCancel,
    SaveSuccess,
    SaveFailed { error: String },

    Quit,
}
