use super::*;

impl App {
    pub(super) fn resolve_variables(&self, input: &str) -> String {
        let mut result = self.resolve_chain_refs(input);
        for (k, v) in &self.extractors.extracted {
            let pattern = format!("{{{{{k}}}}}");
            result = result.replace(&pattern, v);
        }
        let Some(env_id) = &self.env.active_id else {
            return result;
        };
        let Some(env) = self.env.environments.iter().find(|e| e.id == *env_id) else {
            return result;
        };
        for var in &env.variables {
            let pattern = format!("{{{{{}}}}}", var.key);
            result = result.replace(&pattern, &var.value);
        }
        result
    }

    /// Expands `{{$res:RequestName.path.to.field}}` markers using the bodies
    /// of previously sent requests stored in `self.response.last_bodies`.
    fn resolve_chain_refs(&self, input: &str) -> String {
        const OPEN: &str = "{{$res:";
        const CLOSE: &str = "}}";
        let mut out = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(start) = rest.find(OPEN) {
            out.push_str(&rest[..start]);
            let after = &rest[start + OPEN.len()..];
            let Some(end) = after.find(CLOSE) else {
                out.push_str(&rest[start..]);
                return out;
            };
            let expr = &after[..end];
            out.push_str(&self.lookup_chain_expr(expr));
            rest = &after[end + CLOSE.len()..];
        }
        out.push_str(rest);
        out
    }

    fn lookup_chain_expr(&self, expr: &str) -> String {
        let (name, path) = expr
            .split_once('.')
            .map_or((expr, ""), |(n, p)| (n.trim(), p.trim()));
        let Some(body) = self.response.last_bodies.get(name) else {
            return format!("{{{{$res:{expr}}}}}");
        };
        if path.is_empty() {
            return body.clone();
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
            return format!("{{{{$res:{expr}}}}}");
        };
        let mut current = &value;
        for segment in path.split('.') {
            let next = segment
                .parse::<usize>()
                .map_or_else(|_| current.get(segment), |idx| current.get(idx));
            let Some(next) = next else {
                return format!("{{{{$res:{expr}}}}}");
            };
            current = next;
        }
        match current {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    // -- Search --

    pub fn open_search(&mut self) {
        self.response.search_buf = self.response.search.clone();
        self.response.searching = true;
    }

    pub fn confirm_search(&mut self) {
        self.response.search = self.response.search_buf.clone();
        self.response.searching = false;
        self.response.match_idx = 0;
        self.scroll_to_match();
    }

    pub const fn cancel_search(&mut self) {
        self.response.searching = false;
    }

    pub fn clear_search(&mut self) {
        self.response.search.clear();
        self.response.searching = false;
    }

    pub fn next_match(&mut self) {
        if self.response.search.is_empty() {
            return;
        }
        self.response.match_idx += 1;
        self.scroll_to_match();
    }

    pub fn prev_match(&mut self) {
        if self.response.search.is_empty() {
            return;
        }
        self.response.match_idx = self.response.match_idx.saturating_sub(1);
        self.scroll_to_match();
    }

    fn scroll_to_match(&mut self) {
        let body = self.formatted_response_body();
        if self.response.search.is_empty() {
            return;
        }
        let needle = self.response.search.to_ascii_lowercase();
        let mut match_count = 0;
        for (i, line) in body.lines().enumerate() {
            if line.to_ascii_lowercase().contains(&needle) {
                if match_count == self.response.match_idx {
                    self.response.scroll = i as u16;
                    return;
                }
                match_count += 1;
            }
        }
        if match_count > 0 {
            self.response.match_idx = 0;
            self.scroll_to_match();
        }
    }

    pub(super) fn resolve_form_entries(&self) -> Vec<(String, String)> {
        self.request
            .form_editor
            .entries
            .iter()
            .map(|(k, v)| (self.resolve_variables(k), self.resolve_variables(v)))
            .collect()
    }

    // -- Request --
}
