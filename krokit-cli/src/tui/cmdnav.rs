use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: String,
    pub args: Vec<String>,
}

pub struct CommandNav {
    suggestions: Vec<CommandSuggestion>,
    filtered_suggestions: Vec<CommandSuggestion>,
    selected_index: usize,
    is_visible: bool,
    filter_text: String,
    list_state: ListState,
    max_lines: u16,
    view_offset: usize,
}

#[derive(Debug, PartialEq)]
pub enum NavDirection {
    Up,
    Down,
}

impl CommandNav {
    pub fn new() -> Self {
        let mut nav = Self {
            suggestions: Vec::new(),
            filtered_suggestions: Vec::new(),
            selected_index: 0,
            is_visible: false,
            filter_text: String::new(),
            list_state: ListState::default(),
            max_lines: 8,
            view_offset: 0,
        };
        nav.load_commands();
        nav
    }

    fn load_commands(&mut self) {
        // Load commands from App::list_command()
        let commands = Self::get_available_commands();
        self.suggestions = commands.into_iter().map(|((cmd, desc), args)| {
            CommandSuggestion {
                command: cmd,
                description: desc,
                args,
            }
        }).collect();
    }

    fn get_available_commands() -> HashMap<(String, String), Vec<String>> {
        // Import the command list from App
        use crate::tui::App;
        App::list_command()
    }

    pub fn show_suggestions(&mut self, current_text: &str) {
        self.filter_text = current_text.to_string();
        self.update_filtered_suggestions();

        if !self.filtered_suggestions.is_empty() {
            self.is_visible = true;
            self.selected_index = 0;
            self.view_offset = 0;
            self.update_list_state();
        } else {
            self.is_visible = false;
        }
    }

    pub fn hide_suggestions(&mut self) {
        self.is_visible = false;
        self.filter_text.clear();
        self.filtered_suggestions.clear();
        self.selected_index = 0;
        self.view_offset = 0;
    }

    pub fn navigate(&mut self, direction: NavDirection) {
        if !self.is_visible || self.filtered_suggestions.is_empty() {
            return;
        }

        match direction {
            NavDirection::Up => {
                if self.selected_index > 0 { self.selected_index -= 1; }
                else { self.selected_index = self.filtered_suggestions.len().saturating_sub(1); }
                if self.selected_index < self.view_offset { self.view_offset = self.selected_index; }
            }
            NavDirection::Down => {
                if self.selected_index + 1 < self.filtered_suggestions.len() { self.selected_index += 1; }
                else { self.selected_index = 0; self.view_offset = 0; }
                let max = self.max_lines as usize;
                if self.selected_index >= self.view_offset + max { self.view_offset = self.selected_index + 1 - max; }
            }
        }
        self.update_list_state();
    }

    pub fn get_selected_completion(&self) -> Option<String> {
        if !self.is_visible || self.filtered_suggestions.is_empty() {
            return None;
        }

        self.filtered_suggestions.get(self.selected_index)
            .map(|suggestion| suggestion.command.clone())
    }

    pub fn is_showing(&self) -> bool {
        self.is_visible && !self.filtered_suggestions.is_empty()
    }

    fn update_filtered_suggestions(&mut self) {
        if self.filter_text.is_empty() || self.filter_text == "/" {
            self.filtered_suggestions = self.suggestions.clone();
        } else {
            self.filtered_suggestions = self.suggestions
                .iter()
                .filter(|suggestion| {
                    suggestion.command.to_lowercase()
                        .starts_with(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();
        }
        self.selected_index = 0;
        self.view_offset = 0;
    }

    fn update_list_state(&mut self) {
        if self.selected_index < self.filtered_suggestions.len() {
            self.list_state.select(Some(self.selected_index));
        } else {
            self.list_state.select(None);
        }
    }

    pub fn render(&mut self, f: &mut Frame, input_area: Rect) {
        if !self.is_showing() {
            return;
        }

        let frame_area = f.area();

        // Calculate popup size and position
        let popup_height = (self.filtered_suggestions.len() as u16).min(4) + 2; // +2 for borders
        let popup_width = 60.min(input_area.width.saturating_sub(4));

        // Ensure we don't go outside frame boundaries
        let popup_x = input_area.x.min(frame_area.width.saturating_sub(popup_width));

        // Position popup below the input area by default for better visibility
        let popup_y = if input_area.y + input_area.height + popup_height <= frame_area.height {
            // Preferred: position below input area
            input_area.y + input_area.height
        } else if input_area.y >= popup_height {
            // Secondary: position above input area
            input_area.y.saturating_sub(popup_height)
        } else {
            // Fallback: position at bottom of visible area
            frame_area.height.saturating_sub(popup_height)
        };

        // Ensure popup fits within frame boundaries
        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height.min(frame_area.height.saturating_sub(popup_y)),
        };

        // Validate popup area before rendering
        if popup_area.width == 0 || popup_area.height == 0 {
            return;
        }

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Check if we have items to display before creating widgets
        if self.filtered_suggestions.is_empty() || popup_area.height < 3 {
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self.filtered_suggestions
            .iter()
            .map(|suggestion| {
                let args_text = if suggestion.args.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", suggestion.args.join(", "))
                };

                let line = Line::from(vec![
                    Span::styled(
                        suggestion.command.clone(),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::styled(
                        args_text,
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        format!(" - {}", suggestion.description),
                        Style::default().fg(Color::Gray),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        // Create the list widget
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border::ROUNDED)
                    .title("Commands")
                    .border_style(Style::default().fg(Color::Yellow).bold())
                    .title_style(Style::default().fg(Color::Yellow).bold())
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .bold(),
            );

        f.render_stateful_widget(list, popup_area, &mut self.list_state);
    }

    pub fn height(&self) -> u16 {
        if !self.is_showing() { return 0; }
        (self.filtered_suggestions.len() as u16).min(self.max_lines)
    }

    pub fn render_inline(&self, f: &mut Frame, area: Rect) {
        if !self.is_showing() || area.height == 0 { return; }
        let mut lines: Vec<Line> = Vec::new();
        let max = self.height() as usize;
        for (i, sug) in self.filtered_suggestions.iter().skip(self.view_offset).take(max).enumerate() {
            let idx = self.view_offset + i;
            let text = sug.command.clone();
            if idx == self.selected_index {
                lines.push(Line::from(Span::styled(text, Style::default().fg(Color::White).bold())));
            } else {
                lines.push(Line::from(Span::styled(text, Style::default().fg(Color::Gray))));
            }
        }
        let text = Text::from(lines);
        f.render_widget(text, area);
    }

    pub fn selected_has_args(&self) -> bool {
        if !self.is_showing() || self.filtered_suggestions.is_empty() { return false; }
        self.filtered_suggestions
            .get(self.selected_index)
            .map(|s| !s.args.is_empty())
            .unwrap_or(false)
    }
}
