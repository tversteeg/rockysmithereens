use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use miette::{Context, IntoDiagnostic, Result};
use ratatui::{
    prelude::{Buffer, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};

/// Filetree widget state.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FileTreeState {
    /// Current opened folder.
    current_dir: Utf8PathBuf,
    /// Currently highlighted item.
    current: usize,
    /// Folders in directory.
    dirs: Vec<String>,
    /// Files in directory.
    files: Vec<String>,
}

impl FileTreeState {
    /// Create the state from the current working directory.
    pub fn from_current_dir() -> Result<Self> {
        let current_dir = Utf8PathBuf::from_path_buf(
            std::env::current_dir()
                .into_diagnostic()
                .wrap_err("Error getting current directory for filetree widget")?,
        )
        .map_err(|path| miette::miette!("Path '{path:?}' is not valid UTF-8"))?;
        let dirs = Vec::new();
        let files = Vec::new();
        let current = 0;

        let mut this = Self {
            current_dir,
            dirs,
            files,
            current,
        };

        // Read the directory
        this.read_current_dir()?;

        Ok(this)
    }

    /// Handle the key for selecting the items.
    pub fn update(&mut self, key: &KeyEvent) -> Result<()> {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => self.up(),
                KeyCode::Down | KeyCode::Char('j') => self.next(),
                KeyCode::Up | KeyCode::Char('k') => self.previous(),
                KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('l') => {
                    self.select_or_enter()
                }
                _ => Ok(()),
            }?;
        }

        Ok(())
    }

    /// Get the selected file or directory.
    pub fn selected(&self) -> Utf8PathBuf {
        if self.current < self.dirs.len() {
            // A directory is selected
            self.current_dir.join(&self.dirs[self.current])
        } else {
            // A file is selected
            self.current_dir
                .join(&self.files[self.current - self.dirs.len()])
        }
    }

    /// Fill the folders and files from the current directory.
    fn read_current_dir(&mut self) -> Result<()> {
        // Reset the old files and dirs
        self.files.clear();
        self.dirs.clear();
        self.dirs.push("..".to_string());
        self.current = 0;

        // Fill the files and dirs
        for res in self
            .current_dir
            .read_dir_utf8()
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Error reading current directory '{}' for filetree widget",
                    self.current_dir
                )
            })?
        {
            let dir_entry = res.into_diagnostic()?;
            let path = dir_entry.path();
            if let Some(file_name) = path.file_name() {
                if path.is_dir() {
                    self.dirs.push(file_name.to_string());
                } else {
                    self.files.push(file_name.to_string());
                }
            }
        }

        // Sort both by name
        self.files.sort();
        self.dirs.sort();

        Ok(())
    }

    /// Go up a directory if possible.
    fn up(&mut self) -> Result<()> {
        if let Some(higher) = self.current_dir.parent() {
            self.current_dir = higher.to_path_buf();

            self.read_current_dir()?;
        }

        Ok(())
    }

    /// Move the cursor down.
    ///
    /// Wraps around at the bottom.
    fn next(&mut self) -> Result<()> {
        if self.current == self.files.len() + self.dirs.len() - 1 {
            self.current = 0;
        } else {
            self.current += 1;
        }

        Ok(())
    }

    /// Move the cursor up.
    ///
    /// Wraps around at the top.
    fn previous(&mut self) -> Result<()> {
        if self.current == 0 {
            self.current = self.files.len() + self.dirs.len() - 1;
        } else {
            self.current -= 1;
        }

        Ok(())
    }

    /// Select the file or enter the directory.
    fn select_or_enter(&mut self) -> Result<()> {
        // Current is a directory
        if self.current < self.dirs.len() {
            // Go to the directory
            self.current_dir = self.selected();

            self.read_current_dir()?;
        } else {
            // Select the file
            todo!()
        }

        Ok(())
    }
}

/// File-tree widget.
///
/// Can be used to select a file or a directory.
///
/// The widget is always stateful.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct FileTree<'a> {
    /// Block to draw around the widget.
    block: Option<Block<'a>>,
    /// Style used as a base style for the widget.
    style: Style,
    /// Style used to render selected item.
    highlight_style: Style,
    /// Style used to render directory items.
    dir_style: Style,
    /// Style used to render file items.
    file_style: Style,
}

impl<'a> FileTree<'a> {
    /// Create a new filetree widget.
    pub fn new() -> Self {
        let block = None;
        let dir_style = Style::default().add_modifier(Modifier::BOLD);
        let highlight_style = Style::default().add_modifier(Modifier::REVERSED);

        Self {
            block,
            dir_style,
            highlight_style,
            ..Default::default()
        }
    }
}

impl<'a> StatefulWidget for FileTree<'a> {
    type State = FileTreeState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Set the main widget style
        buf.set_style(area, self.style);

        // Calculate the area to render the list, depending on whether we are rendering a wrapping block
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);

                inner_area
            }
            None => area,
        };

        // Do nothing when there's nothing to render
        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        // Draw the directories
        for (j, dir) in state.dirs.iter().enumerate() {
            buf.set_stringn(
                list_area.left(),
                list_area.top().saturating_add(j as u16),
                dir,
                list_area.width as usize,
                if state.current == j {
                    self.highlight_style
                } else {
                    self.dir_style
                },
            );
        }

        // Draw the files
        for (j, file) in state.files.iter().enumerate() {
            buf.set_stringn(
                list_area.left(),
                list_area
                    .top()
                    .saturating_add((j + state.dirs.len()) as u16),
                file,
                list_area.width as usize,
                if state.current == j + state.dirs.len() {
                    self.highlight_style
                } else {
                    self.file_style
                },
            );
        }
    }
}
