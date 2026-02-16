use ratatui::style::{Color, Modifier, Style};

// Solarized Dark palette
// https://github.com/altercation/solarized

// Base monotone colors (dark theme usage)
// const BASE03: Color = Color::Rgb(0, 43, 54);    // background
const BASE02: Color = Color::Rgb(7, 54, 66); //       background highlights
const BASE01: Color = Color::Rgb(88, 110, 117); //    secondary content
// const BASE00: Color = Color::Rgb(101, 123, 131); // (light theme body text)
// const BASE0: Color = Color::Rgb(131, 148, 150);  // body text
const BASE1: Color = Color::Rgb(147, 161, 161); //    emphasized content

// Accent colors
pub const YELLOW: Color = Color::Rgb(181, 137, 0);
// const ORANGE: Color = Color::Rgb(203, 75, 22);
pub const RED: Color = Color::Rgb(220, 50, 47);
// const MAGENTA: Color = Color::Rgb(211, 54, 130);
// const VIOLET: Color = Color::Rgb(108, 113, 196);
pub const BLUE: Color = Color::Rgb(38, 139, 210);
pub const CYAN: Color = Color::Rgb(42, 161, 152);
pub const GREEN: Color = Color::Rgb(133, 153, 0);

// Calendar grid
pub const SELECTED_DATE: Style = Style::new().bg(BASE02).add_modifier(Modifier::BOLD);
pub const TODAY_DATE: Style = Style::new().bg(BASE02).fg(GREEN).add_modifier(Modifier::BOLD);
pub const SUNDAY_STYLE: Style = Style::new().fg(RED).add_modifier(Modifier::BOLD);
pub const SATURDAY_STYLE: Style = Style::new().fg(BLUE).add_modifier(Modifier::BOLD);
pub const WEEKDAY_STYLE: Style = Style::new().fg(BASE1).add_modifier(Modifier::BOLD);
pub const OTHER_MONTH_DATE: Style = Style::new().fg(BASE01);
pub const MONTH_HEADER: Style = Style::new().fg(BASE1).add_modifier(Modifier::BOLD);

// Events
pub const EVENT_TIME: Style = Style::new().fg(GREEN);
pub const EVENT_TITLE: Style = Style::new().add_modifier(Modifier::BOLD);
pub const EVENT_LOCATION: Style = Style::new().fg(CYAN);
pub const SELECTED_EVENT_INDICATOR: Style = Style::new().fg(CYAN);

// Borders
pub const FOCUSED_BORDER: Style = Style::new().fg(BLUE);
pub const UNFOCUSED_BORDER: Style = Style::new().fg(BASE01);

// Help bar
pub const HELP_BAR: Style = Style::new().fg(BASE01);
pub const HELP_KEY: Style = Style::new().fg(YELLOW).add_modifier(Modifier::BOLD);
pub const ERROR_STYLE: Style = Style::new().fg(RED);

// Detail modal
pub const DETAIL_LABEL: Style = Style::new().fg(YELLOW).add_modifier(Modifier::BOLD);
pub const STATUS_CONFIRMED: Style = Style::new().fg(GREEN);
pub const STATUS_TENTATIVE: Style = Style::new().fg(YELLOW);
pub const STATUS_CANCELLED: Style = Style::new().fg(RED);
pub const STATUS_UNKNOWN: Style = Style::new().fg(CYAN);
