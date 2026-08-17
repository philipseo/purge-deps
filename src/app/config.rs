use super::match_rule::MatchRule;
use super::presets::{self, Preset, DEFAULT_PRESETS};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

/// Runtime options for a purge run.
///
/// Example: `Config::new("./apps")` uses the `js` + `frontend` presets under `./apps`.
#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub targets: Vec<String>,
    pub ignore: Vec<String>,
    pub use_gitignore: bool,
    pub dry_run: bool,
    pub presets: Vec<Preset>,
    /// Path-aware rules evaluated after the exact-name HashSet.
    ///
    /// Example: `NameInJsPackage("dist")` from the frontend preset.
    pub extra_rules: Vec<MatchRule>,
}

impl Config {
    /// Default directory names to skip (not deleted, not descended into).
    ///
    /// Example: a `src/node_modules` folder is left alone because `src` is ignored.
    pub fn default_ignore() -> Vec<String> {
        vec![
            ".changeset".to_string(),
            ".git".to_string(),
            ".github".to_string(),
            ".husky".to_string(),
            "src".to_string(),
        ]
    }

    /// Config with default `js` + `frontend` presets and gitignore enabled.
    ///
    /// Example: `Config::new(".")` is what `purge-deps` uses with no flags.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let mut config = Self {
            path: path.into(),
            targets: Vec::new(),
            ignore: Self::default_ignore(),
            use_gitignore: true,
            dry_run: false,
            presets: Vec::new(),
            extra_rules: Vec::new(),
        };
        config.apply_presets(DEFAULT_PRESETS);
        config
    }

    /// Replace `targets` and `extra_rules` with the union of these presets.
    ///
    /// Example: `apply_presets(&[Preset::Js])` is lockfiles + `node_modules` only.
    pub fn apply_presets(&mut self, presets: &[Preset]) {
        self.presets = presets.to_vec();
        let (targets, extra_rules) = presets::collect_rules(presets);
        self.targets = targets;
        self.extra_rules = extra_rules;
    }

    /// Target names as an `O(1)` set for walk matching.
    ///
    /// Example: `target_set().contains(OsStr::new("node_modules"))`.
    pub fn target_set(&self) -> HashSet<OsString> {
        self.targets.iter().map(OsString::from).collect()
    }

    /// Ignore names as an `O(1)` set for walk matching.
    ///
    /// Example: `ignore_set().contains(OsStr::new("src"))`.
    pub fn ignore_set(&self) -> HashSet<OsString> {
        self.ignore.iter().map(OsString::from).collect()
    }

    /// Exact names from `targets` plus [`MatchRule::ExactName`] entries in `extra_rules`.
    ///
    /// Example: `targets` of `node_modules` plus `ExactName(".next")` both land in this set.
    pub fn exact_name_set(&self) -> HashSet<OsString> {
        let mut names = self.target_set();
        for rule in &self.extra_rules {
            if let MatchRule::ExactName(name) = rule {
                names.insert(OsString::from(*name));
            }
        }
        names
    }

    /// Path-aware rules (everything except [`MatchRule::ExactName`]).
    ///
    /// Example: `PathSuffix` / `NameInJsPackage` / `NameWithSibling` are checked
    /// only after the exact-name set misses.
    pub fn path_rules(&self) -> Vec<MatchRule> {
        self.extra_rules
            .iter()
            .copied()
            .filter(|rule| !matches!(rule, MatchRule::ExactName(_)))
            .collect()
    }

    /// Whether this file/folder name should be skipped entirely.
    ///
    /// Example: `should_ignore("src")` is true, so walk does not enter `src/`.
    pub fn should_ignore(&self, name: &str) -> bool {
        self.ignore.iter().any(|s| s == name)
    }

    /// Whether this file/folder name is a deletion target.
    ///
    /// Example: `is_target("node_modules")` is true for the default config.
    pub fn is_target(&self, name: &str) -> bool {
        self.targets.iter().any(|s| s == name)
    }

    /// Append names to ignore unless they are already listed.
    ///
    /// Example: `-i vendor` keeps `.git` / `src` and adds `vendor`.
    pub fn extend_ignore(&mut self, names: impl IntoIterator<Item = String>) {
        for name in names {
            if !self.ignore.contains(&name) {
                self.ignore.push(name);
            }
        }
    }
}

/// Split a comma-separated CLI list, trimming empty entries.
///
/// Example: `" node_modules , , dist "` → `["node_modules", "dist"]`.
pub fn parse_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_list_trims_and_skips_empty() {
        assert_eq!(
            parse_list(" node_modules , , dist "),
            vec!["node_modules", "dist"]
        );
    }

    #[test]
    fn default_config_uses_js_and_frontend() {
        let config = Config::new(".");
        assert!(config.targets.contains(&"node_modules".to_string()));
        assert!(config.targets.contains(&".next".to_string()));
        assert!(config.targets.contains(&".turbo".to_string()));
        assert!(!config.targets.contains(&".expo".to_string()));
        assert!(!config.ignore.contains(&".turbo".to_string()));
        assert!(config
            .extra_rules
            .contains(&MatchRule::NameInJsPackage("dist")));
    }

    #[test]
    fn name_sets_are_hash_lookups() {
        let config = Config::new(".");
        assert!(config
            .target_set()
            .contains(&OsString::from("node_modules")));
        assert!(config.ignore_set().contains(&OsString::from("src")));
        assert!(!config.ignore_set().contains(&OsString::from(".turbo")));
    }
}
