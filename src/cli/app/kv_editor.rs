#[derive(Debug, Clone, Default)]
pub struct KvEditorState {
    pub entries: Vec<(String, String)>,
    pub selected: usize,
    pub editing: bool,
    pub edit_field: usize,
    pub edit_key_buf: String,
    pub edit_value_buf: String,
    pub editing_existing: bool,
}

impl KvEditorState {
    pub fn count(&self) -> usize {
        self.entries.len() + 1
    }

    pub fn move_down(&mut self) {
        let max = self.count().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub const fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn start_add(&mut self) {
        self.edit_key_buf.clear();
        self.edit_value_buf.clear();
        self.edit_field = 0;
        self.editing_existing = false;
        self.editing = true;
    }

    pub fn start_edit(&mut self) {
        if self.selected >= self.entries.len() {
            self.start_add();
            return;
        }
        let (k, v) = &self.entries[self.selected];
        self.edit_key_buf = k.clone();
        self.edit_value_buf = v.clone();
        self.edit_field = 0;
        self.editing_existing = true;
        self.editing = true;
    }

    pub fn confirm_edit(&mut self) {
        if self.edit_key_buf.trim().is_empty() {
            self.editing = false;
            return;
        }
        if self.editing_existing && self.selected < self.entries.len() {
            self.entries[self.selected] = (self.edit_key_buf.clone(), self.edit_value_buf.clone());
        } else {
            self.entries
                .push((self.edit_key_buf.clone(), self.edit_value_buf.clone()));
            self.selected = self.entries.len().saturating_sub(1);
        }
        self.editing = false;
    }

    pub const fn cancel_edit(&mut self) {
        self.editing = false;
    }

    pub fn delete_selected(&mut self) {
        if self.selected < self.entries.len() {
            self.entries.remove(self.selected);
            if self.selected > 0 && self.selected >= self.entries.len() {
                self.selected = self.entries.len().saturating_sub(1);
            }
        }
    }
}
