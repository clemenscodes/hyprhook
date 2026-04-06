use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct WindowInfo {
    class: String,
    title: String,
}

impl WindowInfo {
    pub fn new(class: String, title: String) -> Self {
        Self { class, title }
    }

    pub fn class(&self) -> &str {
        &self.class
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

pub struct State {
    open: HashMap<String, WindowInfo>,
    focused: WindowInfo,
}

impl State {
    pub fn new(open: HashMap<String, WindowInfo>, focused: WindowInfo) -> Self {
        Self { open, focused }
    }

    pub fn insert_open(&mut self, address: String, info: WindowInfo) {
        self.open.insert(address, info);
    }

    pub fn remove_open(&mut self, address: &str) -> Option<WindowInfo> {
        self.open.remove(address)
    }

    pub fn update_focus(&mut self, new: WindowInfo) -> WindowInfo {
        std::mem::replace(&mut self.focused, new)
    }
}
