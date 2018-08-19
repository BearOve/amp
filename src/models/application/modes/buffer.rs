use std::slice::Iter;
use std::fmt;
use std::path::PathBuf;
use util::SelectableVec;
use scribe::Workspace;
use fragment;
use models::application::modes::{SearchSelectMode, SearchSelectConfig};

#[derive(Clone)]
pub struct BufferEntry {
    pub id: usize,
    pub path: Option<PathBuf>,
    search_str: String,
}

impl fragment::matching::AsStr for BufferEntry {
    fn as_str(&self) -> &str {
        &self.search_str
    }
}

impl ::std::fmt::Display for BufferEntry {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "#{} {}", self.id, self.search_str)
    }
}

pub struct BufferMode {
    insert: bool,
    input: String,
    buffers: Vec<BufferEntry>,
    results: SelectableVec<BufferEntry>,
    config: SearchSelectConfig,
}

impl BufferMode {
    pub fn new(workspace: &mut Workspace, config: SearchSelectConfig) -> BufferMode {
        // ToDo: This code assumes the id is _ALWAYS_ valid in a workspace
        let buffers: Vec<_> = workspace.iter_buffers().map(|entry| {
            let id = entry.buffer.id.unwrap();
            let path = entry.get_path().map(|p| PathBuf::from(p));
            let search_str = path.as_ref().map(|p| p.to_string_lossy().into())
                .unwrap_or_else(|| "<not named>".into());
            BufferEntry { id, path, search_str }
        }).collect();

        BufferMode {
            insert: true,
            input: String::new(),
            buffers,
            results: SelectableVec::new(Vec::new()),
            config,
        }
    }
}

impl fmt::Display for BufferMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BUFFER")
    }
}

impl SearchSelectMode<BufferEntry> for BufferMode {
    fn search(&mut self) {
        let results: Vec<_> = if self.input.is_empty() {
            self.buffers
                .iter()
                .take(self.config.max_results)
                .map(|r| r.clone())
                .collect()
        } else {
            fragment::matching::find(
                &self.input,
                &self.buffers,
                self.config.max_results
            ).into_iter().map(|r| r.clone()).collect()
        };

        self.results = SelectableVec::new(results);
    }

    fn query(&mut self) -> &mut String {
        &mut self.input
    }

    fn insert_mode(&self) -> bool {
        self.insert
    }

    fn set_insert_mode(&mut self, insert_mode: bool) {
        self.insert = insert_mode;
    }

    fn results(&self) -> Iter<BufferEntry> {
        self.results.iter()
    }

    fn selection(&self) -> Option<&BufferEntry> {
        self.results.selection()
    }

    fn selected_index(&self) -> usize {
        self.results.selected_index()
    }

    fn select_previous(&mut self) {
        self.results.select_previous();
    }

    fn select_next(&mut self) {
        self.results.select_next();
    }

    fn config(&self) -> &SearchSelectConfig {
        &self.config
    }
    fn message(&mut self) -> Option<String> {
        if !self.results.is_empty() {
            None
        } else if self.input.is_empty() {
            Some(String::from("No buffers are open."))
        } else {
            Some(String::from("No matching entries found."))
        }
    }
}