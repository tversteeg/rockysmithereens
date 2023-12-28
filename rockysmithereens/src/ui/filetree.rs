use camino::Utf8PathBuf;
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
    current: Utf8PathBuf,
    /// Folders in directory.
    dirs: Vec<String>,
    /// Files in directory.
    files: Vec<String>,
}

impl FileTreeState {
    /// Create the state from the current working directory.
    pub fn from_current_dir() -> Result<Self> {
        let current = Utf8PathBuf::from_path_buf(
            std::env::current_dir()
                .into_diagnostic()
                .wrap_err("Error getting current directory for filetree widget")?,
        )
        .map_err(|path| miette::miette!("Path '{path:?}' is not valid UTF-8"))?;
        let dirs = Vec::new();
        let files = Vec::new();

        let mut this = Self {
            current,
            dirs,
            files,
        };

        // Read the directory
        this.read_current_dir()?;

        Ok(this)
    }

    /// Fill the folders and files from the current directory.
    fn read_current_dir(&mut self) -> Result<()> {
        // Reset the old files and dirs
        self.files.clear();
        self.dirs.clear();
        self.dirs.push("..".to_string());

        // Fill the files and dirs
        for res in std::fs::read_dir(&self.current)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Error reading current directory '{}' for filetree widget",
                    self.current
                )
            })?
        {
            let dir_entry = res.into_diagnostic()?;
            let path = Utf8PathBuf::from_path_buf(dir_entry.path())
                .map_err(|path| miette::miette!("Path '{path:?}' is not valid UTF-8"))?;
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

        Self {
            block,
            dir_style,
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
                self.dir_style,
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
                self.file_style,
            );
        }
    }
}
