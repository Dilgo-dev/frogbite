use super::App;

#[derive(Default)]
pub struct DiffState {
    pub snapshot: Option<String>,
    pub popup_open: bool,
    pub scroll: u16,
}

impl App {
    /// Returns the body text of the current response, or `None` if there
    /// isn't one.
    fn current_response_body(&self) -> Option<String> {
        match &self.response.last {
            Some(Ok(r)) => Some(r.body.clone()),
            Some(Err(e)) => Some(e.clone()),
            None => None,
        }
    }

    /// Snapshots the current response, or opens the diff popup if a snapshot
    /// already exists and a current response is available.
    pub fn diff_snapshot_or_open(&mut self) {
        if self.diff.snapshot.is_some() && self.current_response_body().is_some() {
            self.diff.popup_open = true;
            self.diff.scroll = 0;
            return;
        }
        if let Some(body) = self.current_response_body() {
            self.diff.snapshot = Some(body);
            self.response.clipboard_msg = Some("snapshot saved".to_owned());
        }
    }

    pub fn diff_swap(&mut self) {
        let Some(current) = self.current_response_body() else {
            return;
        };
        let Some(snap) = self.diff.snapshot.take() else {
            return;
        };
        self.diff.snapshot = Some(current);
        if let Some(Ok(r)) = self.response.last.as_mut() {
            r.body = snap;
        }
    }

    pub fn diff_clear(&mut self) {
        self.diff.snapshot = None;
        self.diff.popup_open = false;
        self.diff.scroll = 0;
    }

    pub const fn diff_close(&mut self) {
        self.diff.popup_open = false;
        self.diff.scroll = 0;
    }
}
