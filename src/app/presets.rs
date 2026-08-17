use super::match_rule::MatchRule;

/// Named sets of delete rules. Default CLI uses [`DEFAULT_PRESETS`].
///
/// Example: `--preset js,frontend` is the default; `--preset mobile` is opt-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Js,
    Frontend,
    Mobile,
}

/// Presets applied when `--preset` is omitted: JavaScript deps plus frontend caches.
pub const DEFAULT_PRESETS: &[Preset] = &[Preset::Js, Preset::Frontend];

const JS_RULES: &[MatchRule] = &[
    MatchRule::ExactName("node_modules"),
    MatchRule::ExactName("pnpm-lock.yaml"),
    MatchRule::ExactName("yarn.lock"),
    MatchRule::ExactName("package-lock.json"),
    MatchRule::ExactName("bun.lock"),
    MatchRule::ExactName("bun.lockb"),
];

const FRONTEND_RULES: &[MatchRule] = &[
    MatchRule::ExactName(".next"),
    MatchRule::ExactName(".nuxt"),
    MatchRule::ExactName(".output"),
    MatchRule::ExactName(".nitro"),
    MatchRule::ExactName(".svelte-kit"),
    MatchRule::ExactName(".vite"),
    MatchRule::ExactName(".turbo"),
    MatchRule::ExactName(".parcel-cache"),
    MatchRule::PathSuffix(&[".vercel", "output"]),
    MatchRule::NameInJsPackage("dist"),
    MatchRule::NameInJsPackage("build"),
    MatchRule::NameWithSibling("out", &[".next", "next.config.*"]),
];

const MOBILE_RULES: &[MatchRule] = &[
    MatchRule::ExactName(".expo"),
    MatchRule::ExactName(".expo-shared"),
    MatchRule::PathSuffix(&["ios", "Pods"]),
    MatchRule::PathSuffix(&["ios", "build"]),
    MatchRule::PathSuffix(&["android", "build"]),
    MatchRule::PathSuffix(&["android", "app", "build"]),
    MatchRule::PathSuffix(&["android", ".gradle"]),
    MatchRule::PathSuffix(&["android", ".cxx"]),
    MatchRule::PathSuffix(&["android", "app", ".cxx"]),
];

impl Preset {
    /// CLI name for this preset.
    ///
    /// Example: `Preset::Frontend.as_str()` is `"frontend"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Frontend => "frontend",
            Self::Mobile => "mobile",
        }
    }

    /// Rules that belong to this preset.
    ///
    /// Example: `Preset::Js.rules()` includes `ExactName("node_modules")`.
    pub fn rules(self) -> &'static [MatchRule] {
        match self {
            Self::Js => JS_RULES,
            Self::Frontend => FRONTEND_RULES,
            Self::Mobile => MOBILE_RULES,
        }
    }
}

impl std::str::FromStr for Preset {
    type Err = String;

    /// Parse a single preset token.
    ///
    /// Example: `"mobile"` → `Preset::Mobile`; `"foo"` is an error.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "js" => Ok(Self::Js),
            "frontend" => Ok(Self::Frontend),
            "mobile" => Ok(Self::Mobile),
            other => Err(format!(
                "unknown preset '{other}'. Use js, frontend, or mobile"
            )),
        }
    }
}

/// Merge preset rules into exact names (for display / `-e`) and path-aware rules.
///
/// Example: `[Js, Frontend]` yields `node_modules` plus `NameInJsPackage("dist")`.
pub fn collect_rules(presets: &[Preset]) -> (Vec<String>, Vec<MatchRule>) {
    let mut targets = Vec::new();
    let mut extra_rules = Vec::new();

    for preset in presets {
        for rule in preset.rules() {
            match *rule {
                MatchRule::ExactName(name) => {
                    if !targets.iter().any(|existing| existing == name) {
                        targets.push(name.to_string());
                    }
                }
                path_rule => {
                    if !extra_rules.contains(&path_rule) {
                        extra_rules.push(path_rule);
                    }
                }
            }
        }
    }

    (targets, extra_rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_presets_are_js_and_frontend() {
        assert_eq!(DEFAULT_PRESETS, &[Preset::Js, Preset::Frontend]);
    }

    #[test]
    fn js_includes_node_modules_not_expo() {
        let (targets, rules) = collect_rules(&[Preset::Js]);
        assert!(targets.contains(&"node_modules".to_string()));
        assert!(targets.contains(&"bun.lock".to_string()));
        assert!(!targets.contains(&".expo".to_string()));
        assert!(!targets.contains(&".next".to_string()));
        assert!(rules.is_empty());
    }

    #[test]
    fn frontend_includes_next_and_js_package_dist() {
        let (targets, rules) = collect_rules(&[Preset::Frontend]);
        assert!(targets.contains(&".next".to_string()));
        assert!(targets.contains(&".turbo".to_string()));
        assert!(rules.contains(&MatchRule::NameInJsPackage("dist")));
        assert!(rules.contains(&MatchRule::PathSuffix(&[".vercel", "output"])));
        assert!(!targets.contains(&".expo".to_string()));
    }

    #[test]
    fn mobile_is_opt_in() {
        let (targets, rules) = collect_rules(&[Preset::Mobile]);
        assert!(targets.contains(&".expo".to_string()));
        assert!(rules.contains(&MatchRule::PathSuffix(&["ios", "Pods"])));
        assert!(!targets.contains(&"node_modules".to_string()));
    }
}
