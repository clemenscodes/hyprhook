use regex::RegexSet;
use tracing::warn;

/// Pattern used in a RegexSet slot for rules that omit class or title —
/// matches any string so the rule is always a candidate on that axis.
const MATCH_ALL: &str = "(?s:.*)";

pub struct Rule {
    class: Option<String>,
    title: Option<String>,
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
    ) -> Self {
        if class.is_none() && title.is_none() {
            warn!("rule with no class or title filter matches every window event");
        }
        Self {
            class: class.map(str::to_owned),
            title: title.map(str::to_owned),
            on_open,
            on_close,
            on_focus,
            on_unfocus,
        }
    }

    fn class_pattern(&self) -> &str {
        self.class.as_deref().unwrap_or(MATCH_ALL)
    }

    fn title_pattern(&self) -> &str {
        self.title.as_deref().unwrap_or(MATCH_ALL)
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

/// Holds a set of rules with pre-compiled `RegexSet`s for O(text) matching
/// regardless of rule count. Both class and title patterns are compiled into
/// combined automata once at construction; each window event performs a single
/// pass over the class string and a single pass over the title string, then
/// intersects the two hit-sets to find matching rules.
pub struct RuleSet {
    rules: Vec<Rule>,
    class_set: RegexSet,
    title_set: RegexSet,
}

impl RuleSet {
    pub fn new(rules: Vec<Rule>) -> Result<Self, regex::Error> {
        let class_set = RegexSet::new(rules.iter().map(Rule::class_pattern))?;
        let title_set = RegexSet::new(rules.iter().map(Rule::title_pattern))?;
        Ok(Self {
            rules,
            class_set,
            title_set,
        })
    }

    /// Returns all rules whose class **and** title patterns match.
    /// Cost: one automaton scan over `class`, one over `title`, then a
    /// linear walk of the (usually tiny) hit-sets — independent of total
    /// rule count.
    pub fn matching(&self, class: &str, title: &str) -> Vec<&Rule> {
        let class_hits = self.class_set.matches(class);
        let title_hits = self.title_set.matches(title);
        class_hits
            .iter()
            .filter(|&index| title_hits.matched(index))
            .map(|index| &self.rules[index])
            .collect()
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(class: Option<&str>, title: Option<&str>) -> RuleSet {
        RuleSet::new(vec![Rule::new(
            class,
            title,
            vec![],
            vec![],
            vec![],
            vec![],
        )])
        .unwrap()
    }

    fn set_many(specs: &[(Option<&str>, Option<&str>)]) -> RuleSet {
        let rules = specs
            .iter()
            .map(|(c, t)| Rule::new(*c, *t, vec![], vec![], vec![], vec![]))
            .collect();
        RuleSet::new(rules).unwrap()
    }

    #[test]
    fn class_filter_matches_exact() {
        let set = set(Some("^gamescope$"), None);
        assert_eq!(set.matching("gamescope", "anything").len(), 1);
        assert!(set.matching("other", "anything").is_empty());
    }

    #[test]
    fn class_filter_is_a_substring_match_by_default() {
        let set = set(Some("scope"), None);
        assert_eq!(set.matching("gamescope", "anything").len(), 1);
    }

    #[test]
    fn title_filter_matches_exact() {
        let set = set(None, Some("^Counter-Strike 2$"));
        assert_eq!(set.matching("anything", "Counter-Strike 2").len(), 1);
        assert!(set.matching("anything", "Counter-Strike").is_empty());
    }

    #[test]
    fn both_filters_are_anded() {
        let set = set(Some("^gamescope$"), Some("^Counter-Strike 2$"));
        assert_eq!(set.matching("gamescope", "Counter-Strike 2").len(), 1);
        assert!(set.matching("gamescope", "other title").is_empty());
        assert!(set.matching("other class", "Counter-Strike 2").is_empty());
        assert!(set.matching("other class", "other title").is_empty());
    }

    #[test]
    fn no_filters_matches_any_input() {
        let set = set(None, None);
        assert_eq!(set.matching("anything", "anything").len(), 1);
        assert_eq!(set.matching("", "").len(), 1);
    }

    #[test]
    fn invalid_class_regex_is_rejected() {
        assert!(RuleSet::new(vec![Rule::new(
            Some("[invalid"),
            None,
            vec![],
            vec![],
            vec![],
            vec![]
        )])
        .is_err());
    }

    #[test]
    fn invalid_title_regex_is_rejected() {
        assert!(RuleSet::new(vec![Rule::new(
            None,
            Some("[invalid"),
            vec![],
            vec![],
            vec![],
            vec![]
        )])
        .is_err());
    }

    #[test]
    fn only_matching_rules_are_returned() {
        let set = set_many(&[(Some("^foo$"), None), (Some("^bar$"), None)]);
        assert_eq!(set.matching("foo", "title").len(), 1);
        assert!(set.matching("bar", "title")[0].class.as_deref() == Some("^bar$"));
    }

    #[test]
    fn multiple_rules_can_match_simultaneously() {
        let set = set_many(&[(Some("^foo$"), None), (None, None)]);
        assert_eq!(set.matching("foo", "title").len(), 2);
    }

    #[test]
    fn no_match_returns_empty() {
        let set = set_many(&[(Some("^foo$"), None)]);
        assert!(set.matching("bar", "title").is_empty());
    }

    #[test]
    fn empty_rule_set_returns_empty() {
        let set = RuleSet::new(vec![]).unwrap();
        assert!(set.matching("foo", "title").is_empty());
    }

    #[test]
    fn commands_are_stored_and_accessible() {
        let on_open = vec!["obs-cli".to_owned(), "start-recording".to_owned()];
        let on_focus = vec!["hyprctl".to_owned(), "dispatch".to_owned()];
        let rule = Rule::new(
            None,
            None,
            on_open.clone(),
            vec![],
            on_focus.clone(),
            vec![],
        );
        assert_eq!(rule.on_open(), on_open.as_slice());
        assert_eq!(rule.on_focus(), on_focus.as_slice());
        assert!(rule.on_close().is_empty());
        assert!(rule.on_unfocus().is_empty());
    }
}
