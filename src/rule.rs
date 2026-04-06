use regex::Regex;

pub struct Rule {
    class: Option<Regex>,
    title: Option<Regex>,
    on_open: Vec<Vec<String>>,
    on_close: Vec<Vec<String>>,
    on_focus: Vec<Vec<String>>,
    on_unfocus: Vec<Vec<String>>,
}

impl Rule {
    pub fn new(
        class: Option<&str>,
        title: Option<&str>,
        on_open: Vec<Vec<String>>,
        on_close: Vec<Vec<String>>,
        on_focus: Vec<Vec<String>>,
        on_unfocus: Vec<Vec<String>>,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            class: class.map(Regex::new).transpose()?,
            title: title.map(Regex::new).transpose()?,
            on_open,
            on_close,
            on_focus,
            on_unfocus,
        })
    }

    pub fn matches(&self, class: &str, title: &str) -> bool {
        self.class
            .as_ref()
            .is_none_or(|regex| regex.is_match(class))
            && self
                .title
                .as_ref()
                .is_none_or(|regex| regex.is_match(title))
    }

    pub fn matching<'a>(rules: &'a [Self], class: &str, title: &str) -> Vec<&'a Self> {
        rules
            .iter()
            .filter(|rule| rule.matches(class, title))
            .collect()
    }

    pub fn on_open(&self) -> &[Vec<String>] {
        &self.on_open
    }

    pub fn on_close(&self) -> &[Vec<String>] {
        &self.on_close
    }

    pub fn on_focus(&self) -> &[Vec<String>] {
        &self.on_focus
    }

    pub fn on_unfocus(&self) -> &[Vec<String>] {
        &self.on_unfocus
    }
}
