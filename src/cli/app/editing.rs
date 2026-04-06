use super::*;

impl App {
    // -- URL cursor editing --

    pub fn url_insert(&mut self, c: char) {
        self.request.url.insert(self.request.cursor_pos, c);
        self.request.cursor_pos += c.len_utf8();
    }

    pub fn url_backspace(&mut self) {
        if self.request.cursor_pos > 0 {
            let prev = self.request.url[..self.request.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.request.url.remove(prev);
            self.request.cursor_pos = prev;
        }
    }

    pub fn url_delete(&mut self) {
        if self.request.cursor_pos < self.request.url.len() {
            self.request.url.remove(self.request.cursor_pos);
        }
    }

    pub fn url_cursor_left(&mut self) {
        if self.request.cursor_pos > 0 {
            self.request.cursor_pos = self.request.url[..self.request.cursor_pos]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    pub fn url_cursor_right(&mut self) {
        if self.request.cursor_pos < self.request.url.len() {
            self.request.cursor_pos += self.request.url[self.request.cursor_pos..]
                .chars()
                .next()
                .map_or(0, char::len_utf8);
        }
    }

    pub const fn url_cursor_home(&mut self) {
        self.request.cursor_pos = 0;
    }

    pub fn url_cursor_end(&mut self) {
        self.request.cursor_pos = self.request.url.len();
    }

    pub fn finish_url_edit(&mut self) {
        self.request.editing_url = false;
        self.parse_params_from_url();
        self.sync_to_collection();
    }

    // -- Body cursor editing --

    fn body_line_count(&self) -> usize {
        self.request.body.split('\n').count().max(1)
    }

    fn body_line_len(&self, row: usize) -> usize {
        self.request.body.split('\n').nth(row).map_or(0, str::len)
    }

    fn body_cursor_offset(&self) -> usize {
        let mut offset = 0;
        for (i, line) in self.request.body.split('\n').enumerate() {
            if i == self.request.body_row {
                return offset + self.request.body_col.min(line.len());
            }
            offset += line.len() + 1;
        }
        self.request.body.len()
    }

    pub fn body_insert(&mut self, c: char) {
        let offset = self.body_cursor_offset();
        self.request.body.insert(offset, c);
        self.request.body_col += c.len_utf8();
    }

    pub fn body_insert_newline(&mut self) {
        let offset = self.body_cursor_offset();
        self.request.body.insert(offset, '\n');
        self.request.body_row += 1;
        self.request.body_col = 0;
    }

    pub fn body_insert_tab(&mut self) {
        let offset = self.body_cursor_offset();
        self.request.body.insert_str(offset, "  ");
        self.request.body_col += 2;
    }

    pub fn body_backspace(&mut self) {
        if self.request.body_col > 0 {
            let offset = self.body_cursor_offset();
            self.request.body.remove(offset - 1);
            self.request.body_col -= 1;
        } else if self.request.body_row > 0 {
            let prev_len = self.body_line_len(self.request.body_row - 1);
            let offset = self.body_cursor_offset();
            self.request.body.remove(offset - 1);
            self.request.body_row -= 1;
            self.request.body_col = prev_len;
        }
    }

    pub fn body_delete(&mut self) {
        let offset = self.body_cursor_offset();
        if offset < self.request.body.len() {
            self.request.body.remove(offset);
        }
    }

    pub fn body_cursor_left(&mut self) {
        if self.request.body_col > 0 {
            self.request.body_col -= 1;
        } else if self.request.body_row > 0 {
            self.request.body_row -= 1;
            self.request.body_col = self.body_line_len(self.request.body_row);
        }
    }

    pub fn body_cursor_right(&mut self) {
        let line_len = self.body_line_len(self.request.body_row);
        if self.request.body_col < line_len {
            self.request.body_col += 1;
        } else if self.request.body_row + 1 < self.body_line_count() {
            self.request.body_row += 1;
            self.request.body_col = 0;
        }
    }

    pub fn body_cursor_up(&mut self) {
        if self.request.body_row > 0 {
            self.request.body_row -= 1;
            self.request.body_col = self
                .request
                .body_col
                .min(self.body_line_len(self.request.body_row));
        }
    }

    pub fn body_cursor_down(&mut self) {
        if self.request.body_row + 1 < self.body_line_count() {
            self.request.body_row += 1;
            self.request.body_col = self
                .request
                .body_col
                .min(self.body_line_len(self.request.body_row));
        }
    }

    pub const fn body_cursor_home(&mut self) {
        self.request.body_col = 0;
    }

    pub fn body_cursor_end(&mut self) {
        self.request.body_col = self.body_line_len(self.request.body_row);
    }

    pub fn enter_body_edit(&mut self) {
        self.request.editing_body = true;
        let count = self.body_line_count();
        self.request.body_row = count.saturating_sub(1);
        self.request.body_col = self.body_line_len(self.request.body_row);
    }

    pub fn finish_body_edit(&mut self) {
        self.request.editing_body = false;
        self.sync_to_collection();
    }
}
