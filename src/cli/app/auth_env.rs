use super::*;

impl App {
    // -- Auth --

    pub const AUTH_TYPES: &'static [&'static str] = &["None", "Bearer", "Basic", "API Key"];

    pub const fn auth_type_index(&self) -> usize {
        match &self.auth.config {
            Auth::None => 0,
            Auth::Bearer { .. } => 1,
            Auth::Basic { .. } => 2,
            Auth::ApiKey { .. } => 3,
        }
    }

    pub fn select_auth_type(&mut self) {
        self.auth.selecting_type = false;
        let new_auth = match self.auth.type_selected {
            1 => {
                let token = if let Auth::Bearer { token } = &self.auth.config {
                    token.clone()
                } else {
                    String::new()
                };
                Auth::Bearer { token }
            }
            2 => {
                let (username, password) =
                    if let Auth::Basic { username, password } = &self.auth.config {
                        (username.clone(), password.clone())
                    } else {
                        (String::new(), String::new())
                    };
                Auth::Basic { username, password }
            }
            3 => {
                let (header, value) = if let Auth::ApiKey { header, value } = &self.auth.config {
                    (header.clone(), value.clone())
                } else {
                    ("X-API-Key".to_owned(), String::new())
                };
                Auth::ApiKey { header, value }
            }
            _ => Auth::None,
        };
        self.auth.config = new_auth;
        self.sync_to_collection();

        if self.auth.config != Auth::None {
            self.open_auth_edit();
        }
    }

    pub fn open_auth_edit(&mut self) {
        self.auth.field = 0;
        match &self.auth.config {
            Auth::Bearer { token } => {
                self.auth.buf_a = token.clone();
                self.auth.buf_b.clear();
            }
            Auth::Basic { username, password } => {
                self.auth.buf_a = username.clone();
                self.auth.buf_b = password.clone();
            }
            Auth::ApiKey { header, value } => {
                self.auth.buf_a = header.clone();
                self.auth.buf_b = value.clone();
            }
            Auth::None => return,
        }
        self.auth.editing = true;
    }

    pub fn confirm_auth_edit(&mut self) {
        match &self.auth.config {
            Auth::Bearer { .. } => {
                self.auth.config = Auth::Bearer {
                    token: self.auth.buf_a.clone(),
                };
            }
            Auth::Basic { .. } => {
                self.auth.config = Auth::Basic {
                    username: self.auth.buf_a.clone(),
                    password: self.auth.buf_b.clone(),
                };
            }
            Auth::ApiKey { .. } => {
                self.auth.config = Auth::ApiKey {
                    header: self.auth.buf_a.clone(),
                    value: self.auth.buf_b.clone(),
                };
            }
            Auth::None => {}
        }
        self.auth.editing = false;
        self.sync_to_collection();
    }

    pub(super) fn apply_auth_headers(&self, headers: &mut HashMap<String, String>) {
        match &self.auth.config {
            Auth::None => {}
            Auth::Bearer { token } => {
                let resolved = self.resolve_variables(token);
                headers.insert("Authorization".to_owned(), format!("Bearer {resolved}"));
            }
            Auth::Basic { username, password } => {
                let resolved_user = self.resolve_variables(username);
                let resolved_pass = self.resolve_variables(password);
                let encoded =
                    crate::curl::base64(format!("{resolved_user}:{resolved_pass}").as_bytes());
                headers.insert("Authorization".to_owned(), format!("Basic {encoded}"));
            }
            Auth::ApiKey { header, value } => {
                let resolved_header = self.resolve_variables(header);
                let resolved_value = self.resolve_variables(value);
                headers.insert(resolved_header, resolved_value);
            }
        }
    }

    // -- Environments --

    pub fn save_environments(&self) {
        let data = environments::EnvironmentData {
            environments: self.env.environments.clone(),
            active_id: self.env.active_id.clone(),
        };
        environments::save(&data);
    }

    pub fn active_env_name(&self) -> Option<&str> {
        let id = self.env.active_id.as_ref()?;
        self.env
            .environments
            .iter()
            .find(|e| e.id == *id)
            .map(|e| e.name.as_str())
    }

    pub const fn open_env_popup(&mut self) {
        self.env.popup_selected = 0;
        self.env.popup_open = true;
    }

    pub fn env_popup_count(&self) -> usize {
        self.env.environments.len() + 1
    }

    pub fn select_env_from_popup(&mut self) {
        if self.env.popup_selected == 0 {
            self.env.active_id = None;
        } else {
            let idx = self.env.popup_selected - 1;
            if let Some(env) = self.env.environments.get(idx) {
                self.env.active_id = Some(env.id.clone());
            }
        }
        self.save_environments();
        self.env.popup_open = false;
    }

    pub fn create_environment(&mut self) {
        let env = Environment {
            id: collections::new_id(),
            name: String::new(),
            variables: Vec::new(),
        };
        self.env.environments.push(env);
        self.env.popup_selected = self.env.environments.len();
        self.env.name_buffer = String::new();
        self.env.renaming = true;
        self.save_environments();
    }

    pub fn delete_env_from_popup(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if idx < self.env.environments.len() {
            let removed_id = self.env.environments[idx].id.clone();
            self.env.environments.remove(idx);
            if self.env.active_id.as_deref() == Some(&removed_id) {
                self.env.active_id = None;
            }
            let max = self.env_popup_count().saturating_sub(1);
            if self.env.popup_selected > max {
                self.env.popup_selected = max;
            }
            self.save_environments();
        }
    }

    pub fn start_env_rename(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if let Some(env) = self.env.environments.get(idx) {
            self.env.name_buffer = env.name.clone();
            self.env.renaming = true;
        }
    }

    pub fn confirm_env_rename(&mut self) {
        if self.env.popup_selected == 0 {
            self.env.renaming = false;
            return;
        }
        let idx = self.env.popup_selected - 1;
        if self.env.name_buffer.trim().is_empty() {
            self.cancel_env_rename();
            return;
        }
        if let Some(env) = self.env.environments.get_mut(idx) {
            env.name.clone_from(&self.env.name_buffer);
        }
        self.env.renaming = false;
        self.save_environments();
    }

    pub fn cancel_env_rename(&mut self) {
        if self.env.popup_selected > 0 {
            let idx = self.env.popup_selected - 1;
            if idx < self.env.environments.len() && self.env.environments[idx].name.is_empty() {
                self.env.environments.remove(idx);
                let max = self.env_popup_count().saturating_sub(1);
                if self.env.popup_selected > max {
                    self.env.popup_selected = max;
                }
                self.save_environments();
            }
        }
        self.env.renaming = false;
    }

    pub fn open_env_import(&mut self) {
        self.env.import.buffer = String::new();
        self.env.import.error = false;
        self.env.import.open = true;
    }

    pub fn confirm_env_import(&mut self) {
        let path = std::path::Path::new(self.env.import.buffer.trim());
        let Some(vars) = environments::parse_dotenv(path) else {
            self.env.import.error = true;
            return;
        };

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("dotenv")
            .to_owned();

        let env = Environment {
            id: collections::new_id(),
            name,
            variables: vars,
        };
        self.env.environments.push(env);
        self.env.popup_selected = self.env.environments.len();
        self.save_environments();
        self.env.import.open = false;
    }

    pub fn open_env_editor(&mut self) {
        if self.env.popup_selected == 0 {
            return;
        }
        let idx = self.env.popup_selected - 1;
        if let Some(env) = self.env.environments.get(idx) {
            self.env.editor.id = env.id.clone();
            self.env.editor.selected = 0;
            self.env.popup_open = false;
            self.env.editor.open = true;
        }
    }

    fn edited_env(&self) -> Option<&Environment> {
        self.env
            .environments
            .iter()
            .find(|e| e.id == self.env.editor.id)
    }

    pub fn env_editor_count(&self) -> usize {
        self.edited_env().map_or(1, |e| e.variables.len() + 1)
    }

    pub fn start_add_var(&mut self) {
        self.env.editor.var_key_buffer.clear();
        self.env.editor.var_value_buffer.clear();
        self.env.editor.var_field = 0;
        self.env.editor.editing_var = true;
    }

    pub fn start_edit_var(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected >= env.variables.len() {
            self.start_add_var();
            return;
        }
        self.env.editor.var_key_buffer = env.variables[self.env.editor.selected].key.clone();
        self.env.editor.var_value_buffer = env.variables[self.env.editor.selected].value.clone();
        self.env.editor.var_field = 0;
        self.env.editor.editing_var = true;
    }

    pub fn confirm_var_edit(&mut self) {
        if self.env.editor.var_key_buffer.trim().is_empty() {
            self.env.editor.editing_var = false;
            return;
        }
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            let var = &mut env.variables[self.env.editor.selected];
            var.key.clone_from(&self.env.editor.var_key_buffer);
            var.value.clone_from(&self.env.editor.var_value_buffer);
        } else {
            env.variables.push(Variable {
                key: self.env.editor.var_key_buffer.clone(),
                value: self.env.editor.var_value_buffer.clone(),
                secret: false,
            });
        }
        self.env.editor.editing_var = false;
        self.save_environments();
    }

    pub fn delete_var(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            env.variables.remove(self.env.editor.selected);
            let max = env.variables.len();
            if self.env.editor.selected > max {
                self.env.editor.selected = max;
            }
            self.save_environments();
        }
    }

    pub fn toggle_var_secret(&mut self) {
        let id = self.env.editor.id.clone();
        let Some(env) = self.env.environments.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if self.env.editor.selected < env.variables.len() {
            let var = &mut env.variables[self.env.editor.selected];
            var.secret = !var.secret;
            self.save_environments();
        }
    }
}
