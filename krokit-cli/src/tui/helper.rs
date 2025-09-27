use ansi_to_tui::IntoText;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Widget},
    Frame,
};

pub struct HelpArea;

impl HelpArea {
    fn helper_msg(&self) -> String {
        [
            "  ? to print help      tap esc twice to clear input",
            "                       tap esc while agent is running to cancel",
            "  krokit auth          change provider",
            "  See Available Commands with /:",
        ]
        .join("\n")
        .to_string()
    }
}

impl HelpArea {
    pub fn height(&self) -> u16 {
        8 // content (4 general help lines + 1 blank + 1 header + 2 command lines)
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        let helper_text = self.helper_msg();
        let x = helper_text.into_text().unwrap();
        // Make help text more visible
        let x = x.style(Style::default().fg(Color::White));
        f.render_widget(x, area);
    }
}
