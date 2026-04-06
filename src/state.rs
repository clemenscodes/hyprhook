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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn empty_state() -> State {
        State::new(HashMap::new(), WindowInfo::default())
    }

    fn info(class: &str, title: &str) -> WindowInfo {
        WindowInfo::new(class.to_owned(), title.to_owned())
    }

    #[test]
    fn insert_and_remove_roundtrip() {
        let mut state = empty_state();
        state.insert_open("0x1".to_owned(), info("kitty", "~/"));
        let removed = state.remove_open("0x1").unwrap();
        assert_eq!(removed.class(), "kitty");
        assert_eq!(removed.title(), "~/");
    }

    #[test]
    fn remove_unknown_address_returns_none() {
        let mut state = empty_state();
        assert!(state.remove_open("0xdead").is_none());
    }

    #[test]
    fn remove_is_non_repeatable() {
        let mut state = empty_state();
        state.insert_open("0x1".to_owned(), info("kitty", "~/"));
        state.remove_open("0x1");
        assert!(state.remove_open("0x1").is_none());
    }

    #[test]
    fn update_focus_returns_previous_focused() {
        let mut state = State::new(HashMap::new(), info("firefox", "GitHub"));
        let previous = state.update_focus(info("kitty", "~/"));
        assert_eq!(previous.class(), "firefox");
        assert_eq!(previous.title(), "GitHub");
    }

    #[test]
    fn update_focus_installs_new_focused() {
        let mut state = empty_state();
        state.update_focus(info("firefox", "GitHub"));
        let previous = state.update_focus(info("kitty", "~/"));
        assert_eq!(previous.class(), "firefox");
    }

    #[test]
    fn initial_state_can_be_seeded_with_open_windows() {
        let mut open = HashMap::new();
        open.insert("0x1".to_owned(), info("kitty", "~/"));
        open.insert("0x2".to_owned(), info("firefox", "GitHub"));
        let mut state = State::new(open, WindowInfo::default());
        assert!(state.remove_open("0x1").is_some());
        assert!(state.remove_open("0x2").is_some());
        assert!(state.remove_open("0x3").is_none());
    }
}
