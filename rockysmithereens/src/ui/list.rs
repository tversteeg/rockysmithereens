use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    text::Text,
    widgets::{List, ListItem, ListState},
};

/// A list holding an array of items that can be selected.
pub struct StatefulList {
    /// Tui state.
    pub state: ListState,
    /// All items.
    pub items: Vec<String>,
}

impl StatefulList {
    /// Create a new list with the specified items.
    pub fn with_items(items: &[&str]) -> StatefulList {
        let items = items.iter().map(|s| s.to_string()).collect();
        let state = ListState::default().with_selected(Some(0));

        StatefulList { state, items }
    }

    /// Handle the key for selecting the items.
    pub fn update(&mut self, key: &KeyEvent) -> Option<String> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.next(),
                KeyCode::Up | KeyCode::Char('k') => self.previous(),
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('l') => {
                    // Return the selected line
                    return Some(self.items[self.state.selected().unwrap_or_default()].clone());
                }
                _ => (),
            }
        }

        None
    }

    /// Select the next item.
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Select the previous item.
    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}
