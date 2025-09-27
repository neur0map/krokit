use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::Widget,
    Frame,
};
use ignore::WalkBuilder;

pub struct FileNav {
    all_files: Option<Vec<String>>, // relative paths
    pub visible: bool,
    pub filtered: Vec<String>,
    pub selected: usize,
    pub filter_text: String,
    max_lines: u16,
    view_offset: usize,
}

impl FileNav {
    pub fn new() -> Self {
        Self {
            all_files: None,
            visible: false,
            filtered: Vec::new(),
            selected: 0,
            filter_text: String::new(),
            max_lines: 8,
            view_offset: 0,
        }
    }

    fn ensure_index(&mut self) {
        if self.all_files.is_some() { return; }

        let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut files: Vec<String> = Vec::new();
        let walker = WalkBuilder::new(&root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .follow_links(false)
            .build();

        for result in walker {
            if let Ok(entry) = result {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(rel) = path.strip_prefix(&root) {
                        if let Some(s) = rel.to_str() {
                            files.push(s.replace('\\', "/"));
                        }
                    }
                }
            }
        }

        // Basic sort for stability
        files.sort();
        self.all_files = Some(files);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.filtered.clear();
        self.filter_text.clear();
        self.selected = 0;
        self.view_offset = 0;
    }

    pub fn is_showing(&self) -> bool {
        self.visible && !self.filtered.is_empty()
    }

    pub fn update_filter(&mut self, prefix: &str) {
        self.ensure_index();
        self.filter_text = prefix.to_string();

        let needle = prefix.to_lowercase();
        let all = self.all_files.as_ref().expect("file index should be initialized");

        // Simple ranking: filename starts-with > path contains
        let mut starts: Vec<&String> = Vec::new();
        let mut contains: Vec<&String> = Vec::new();

        for p in all.iter() {
            let p_lower = p.to_lowercase();
            if let Some(fname) = p_lower.rsplit('/').next() {
                if fname.starts_with(&needle) {
                    starts.push(p);
                    continue;
                }
            }
            if p_lower.contains(&needle) {
                contains.push(p);
            }
        }

        let mut combined: Vec<String> = starts.into_iter().chain(contains.into_iter()).take(500).cloned().collect();
        self.filtered = combined;
        self.selected = 0;
        self.view_offset = 0;
        self.visible = !self.filtered.is_empty();
    }

    pub fn height(&self) -> u16 {
        if !self.is_showing() { return 0; }
        (self.filtered.len() as u16).min(self.max_lines)
    }

    pub fn move_up(&mut self) { 
        if !self.is_showing() { return; }
        if self.selected == 0 { 
            self.selected = self.filtered.len().saturating_sub(1);
        } else { 
            self.selected -= 1; 
        }
        // adjust view
        if self.selected < self.view_offset {
            self.view_offset = self.selected;
        }
    }

    pub fn move_down(&mut self) {
        if !self.is_showing() { return; }
        if self.selected + 1 >= self.filtered.len() { 
            self.selected = 0; 
        } else { 
            self.selected += 1; 
        }
        // adjust view
        let max = self.max_lines as usize;
        if self.selected >= self.view_offset + max {
            self.view_offset = self.selected + 1 - max;
        }
    }

    pub fn page_up(&mut self) {
        if !self.is_showing() { return; }
        let page = self.max_lines as usize;
        if self.selected >= page {
            self.selected -= page;
        } else {
            self.selected = 0;
        }
        if self.view_offset >= page {
            self.view_offset -= page;
        } else {
            self.view_offset = 0;
        }
    }

    pub fn page_down(&mut self) {
        if !self.is_showing() { return; }
        let page = self.max_lines as usize;
        let len = self.filtered.len();
        self.selected = (self.selected + page).min(len.saturating_sub(1));
        let max = self.max_lines as usize;
        if self.selected >= self.view_offset + max {
            let desired = self.selected + 1 - max;
            self.view_offset = desired.min(len.saturating_sub(max));
        }
    }

    pub fn selected_value(&self) -> Option<&str> {
        if !self.is_showing() { return None; }
        self.filtered.get(self.selected).map(|s| s.as_str())
    }

    pub fn draw(&self, f: &mut Frame, area: Rect) {
        if !self.is_showing() || area.height == 0 { return; }
        let mut lines: Vec<Line> = Vec::new();
        let max = self.height() as usize;
        for (i, path) in self.filtered.iter().skip(self.view_offset).take(max).enumerate() {
            let idx = self.view_offset + i;
            if idx == self.selected {
                lines.push(Line::from(Span::styled(path.clone(), Style::default().fg(Color::White).bold())));
            } else {
                lines.push(Line::from(Span::styled(path.clone(), Style::default().fg(Color::Gray))));
            }
        }
        let text = Text::from(lines);
        f.render_widget(text, area);
    }
}
