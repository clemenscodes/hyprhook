use regex::Regex;
use tracing::warn;

pub struct Rule {
    class: Option<Regex>,
    title: Option<Regex>,
    on_open: Vec<String>,
    on_close: Vec<String>,
    on_focus: Vec<String>,
    on_unfocus: Vec<String>,
}

impl Rule {
    pub fn new(
        class: Option<&str>,
        title: Option<&str>,
        on_open: Vec<String>,
        on_close: Vec<String>,
        on_focus: Vec<String>,
        on_unfocus: Vec<String>,
    ) -> Result<Self, regex::Error> {
        if class.is_none() && title.is_none() {
            warn!("rule with no class or title filter matches every window event");
        }
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

    pub fn on_open(&self) -> &[String] {
        &self.on_open
    }

    pub fn on_close(&self) -> &[String] {
        &self.on_close
    }

    pub fn on_focus(&self) -> &[String] {
        &self.on_focus
    }

    pub fn on_unfocus(&self) -> &[String] {
        &self.on_unfocus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(class: Option<&str>, title: Option<&str>) -> Rule {
        Rule::new(class, title, vec![], vec![], vec![], vec![]).unwrap()
    }

    fn rule_with_commands(
        class: Option<&str>,
        title: Option<&str>,
        on_open: Vec<String>,
        on_close: Vec<String>,
        on_focus: Vec<String>,
        on_unfocus: Vec<String>,
    ) -> Rule {
        Rule::new(class, title, on_open, on_close, on_focus, on_unfocus).unwrap()
    }

    #[test]
    fn class_filter_matches_exact() {
        let rule = rule(Some("^gamescope$"), None);
        assert!(rule.matches("gamescope", "anything"));
        assert!(!rule.matches("other", "anything"));
    }

    #[test]
    fn class_filter_is_a_substring_match_by_default() {
        let rule = rule(Some("scope"), None);
        assert!(rule.matches("gamescope", "anything"));
    }

    #[test]
    fn title_filter_matches_exact() {
        let rule = rule(None, Some("^Counter-Strike 2$"));
        assert!(rule.matches("anything", "Counter-Strike 2"));
        assert!(!rule.matches("anything", "Counter-Strike"));
    }

    #[test]
    fn both_filters_are_anded() {
        let rule = rule(Some("^gamescope$"), Some("^Counter-Strike 2$"));
        assert!(rule.matches("gamescope", "Counter-Strike 2"));
        assert!(!rule.matches("gamescope", "other title"));
        assert!(!rule.matches("other class", "Counter-Strike 2"));
        assert!(!rule.matches("other class", "other title"));
    }

    #[test]
    fn no_filters_matches_any_input() {
        let rule = rule(None, None);
        assert!(rule.matches("anything", "anything"));
        assert!(rule.matches("", ""));
    }

    #[test]
    fn invalid_class_regex_returns_error() {
        let result = Rule::new(Some("[invalid"), None, vec![], vec![], vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_title_regex_returns_error() {
        let result = Rule::new(None, Some("[invalid"), vec![], vec![], vec![], vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn matching_returns_only_matched_rules() {
        let rules = vec![rule(Some("^foo$"), None), rule(Some("^bar$"), None)];
        let matched = Rule::matching(&rules, "foo", "title");
        assert_eq!(matched.len(), 1);
        assert!(matched[0].matches("foo", "title"));
    }

    #[test]
    fn matching_returns_multiple_rules_when_several_match() {
        let rules = vec![rule(Some("^foo$"), None), rule(None, None)];
        let matched = Rule::matching(&rules, "foo", "title");
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn matching_returns_empty_when_nothing_matches() {
        let rules = vec![rule(Some("^foo$"), None)];
        let matched = Rule::matching(&rules, "bar", "title");
        assert!(matched.is_empty());
    }

    #[test]
    fn matching_against_empty_rules_returns_empty() {
        let matched = Rule::matching(&[], "foo", "title");
        assert!(matched.is_empty());
    }

    #[test]
    fn commands_are_stored_and_accessible() {
        let open = vec!["obs-cli".to_owned(), "start-recording".to_owned()];
        let focus = vec!["hyprctl".to_owned(), "dispatch".to_owned()];
        let rule = rule_with_commands(None, None, open.clone(), vec![], focus.clone(), vec![]);
        assert_eq!(rule.on_open(), open.as_slice());
        assert_eq!(rule.on_focus(), focus.as_slice());
        assert!(rule.on_close().is_empty());
        assert!(rule.on_unfocus().is_empty());
    }
}
