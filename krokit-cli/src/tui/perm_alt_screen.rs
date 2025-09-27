use std::io::{self, stdout, Write};
use crossterm::cursor::MoveTo;
use crossterm::event::{self, EnableFocusChange, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, ExecutableCommand};
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use futures::StreamExt;
use tokio::time::{sleep, Duration};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal
};
use krokit_core::agent::events::PermissionRequest;

use super::perm::{PermissionWidget, PermissionModalAction};

pub struct AlternateScreenPermissionModal<'a> {
    widget: PermissionWidget<'a>,
}

impl AlternateScreenPermissionModal<'_> {
    pub fn new(widget: &PermissionWidget) -> io::Result<Self> {
        Ok(Self {
            widget: PermissionWidget::new(
                widget.request_id.clone(),
                widget.request.clone(),
                widget.remaining_perms)
        })
    }
    
    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        self.widget.draw(frame, area);
    }


    pub async fn run(&mut self) -> io::Result<PermissionModalAction> {
        // Enter alternate screen and enable mouse capture
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;

        let result = self.run_modal().await;

        // Always clean up - leave alternate screen and disable mouse capture
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = stdout().flush();
        
        // Small delay to ensure terminal state is properly restored
        sleep(Duration::from_millis(50)).await;

        result
    }

    async fn run_modal(&mut self) -> io::Result<PermissionModalAction> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut reader = event::EventStream::new();
        
        loop {
            terminal.draw(|frame| {
                let area = frame.area();
                self.widget.draw(frame, area);
            })?;

            if let Some(Ok(event)) = reader.next().await {
                match event {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        // Handle Ctrl+C to exit
                        if matches!(key_event.code, KeyCode::Char('c')) 
                            && key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            // Treat Ctrl+C as Escape (Deny)
                            return Ok(PermissionModalAction::Response {
                                request_id: "".to_string(), // We'll fix this access later
                                choice: krokit_core::agent::PermissionResponse::Deny,
                            });
                        }

                        // Pass all key events to the widget
                        let action = self.widget.handle_key_event(key_event).await;
                        if !matches!(action, PermissionModalAction::Nope) {
                            return Ok(action);
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        let _ = self.widget.handle_mouse_event(mouse_event).await;
                    }
                    Event::Resize(..) => {
                        // Terminal was resized, redraw on next iteration
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Drop for AlternateScreenPermissionModal<'_> {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen);
        let _ = stdout().flush();
    }
}
