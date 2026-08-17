use std::ffi::OsStr;
use std::path::Path;

/// How a path can qualify as a deletion target.
///
/// Exact names are usually stored in [`super::config::Config::targets`] for
/// `O(1)` lookup. These variants cover path-aware cases that a basename set cannot.
///
/// Example: `MatchRule::PathSuffix(&["android", "app", "build"])` matches
/// `apps/mobile/android/app/build` but not `docs/build`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchRule {
    /// Basename equals this name.
    ///
    /// Example: `ExactName("node_modules")` matches `packages/web/node_modules`.
    ExactName(&'static str),
    /// Trailing path components match, in order.
    ///
    /// Example: `PathSuffix(&["ios", "Pods"])` matches `app/ios/Pods`.
    PathSuffix(&'static [&'static str]),
    /// Basename match whose parent contains `package.json`, excluding native trees.
    ///
    /// Example: `NameInJsPackage("dist")` matches `web/dist` next to `web/package.json`,
    /// not `docs/dist` and not `android/app/build`.
    NameInJsPackage(&'static str),
    /// Basename match whose parent also has one of these sibling names.
    ///
    /// A sibling ending in `*` is a prefix (e.g. `next.config.*`).
    ///
    /// Example: `NameWithSibling("out", &[".next", "next.config.*"])` matches
    /// `app/out` when `app/next.config.ts` exists.
    NameWithSibling(&'static str, &'static [&'static str]),
}

/// Names that must never be deleted, even if a rule or `-t` lists them.
///
/// Example: `is_protected(OsStr::new(".git"))` is true.
pub fn is_protected(name: &OsStr) -> bool {
    name == ".git"
}

impl MatchRule {
    /// Whether `path` satisfies this rule.
    ///
    /// Example: `PathSuffix(&["android", "build"]).matches("app/android/build")` is true.
    pub fn matches(&self, path: &Path) -> bool {
        match *self {
            Self::ExactName(name) => file_name_eq(path, name),
            Self::PathSuffix(suffix) => path_has_suffix(path, suffix),
            Self::NameInJsPackage(name) => {
                file_name_eq(path, name)
                    && !is_native_project_path(path)
                    && parent_has_package_json(path)
            }
            Self::NameWithSibling(name, siblings) => {
                file_name_eq(path, name) && parent_has_any_sibling(path, siblings)
            }
        }
    }
}

fn file_name_eq(path: &Path, name: &str) -> bool {
    path.file_name().is_some_and(|file_name| file_name == name)
}

/// True when the last components of `path` equal `suffix`.
///
/// Example: `app/android/app/build` has suffix `["android", "app", "build"]`.
fn path_has_suffix(path: &Path, suffix: &[&str]) -> bool {
    let mut path_iter = path.iter().rev();
    suffix.iter().rev().all(|expected| {
        path_iter
            .next()
            .is_some_and(|component| component == OsStr::new(*expected))
    })
}

/// True when any path component is a native project folder (`android` or `ios`).
///
/// Example: `mobile/android/app/build` is native; `packages/web/build` is not.
fn is_native_project_path(path: &Path) -> bool {
    path.iter()
        .any(|component| component == "android" || component == "ios")
}

fn parent_has_package_json(path: &Path) -> bool {
    path.parent()
        .is_some_and(|parent| parent.join("package.json").is_file())
}

fn parent_has_any_sibling(path: &Path, siblings: &[&str]) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };

    siblings.iter().any(|sibling| {
        if let Some(prefix) = sibling.strip_suffix('*') {
            prefix_exists_in_dir(parent, prefix)
        } else {
            parent.join(sibling).exists()
        }
    })
}

fn prefix_exists_in_dir(parent: &Path, prefix: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(prefix))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn exact_name_matches_basename() {
        let rule = MatchRule::ExactName("node_modules");
        assert!(rule.matches(Path::new("packages/web/node_modules")));
        assert!(!rule.matches(Path::new("packages/web/src")));
    }

    #[test]
    fn path_suffix_matches_trailing_components() {
        let rule = MatchRule::PathSuffix(&["android", "app", "build"]);
        assert!(rule.matches(Path::new("apps/mobile/android/app/build")));
        assert!(!rule.matches(Path::new("docs/build")));
        assert!(!rule.matches(Path::new("android/build")));
    }

    #[test]
    fn name_in_js_package_requires_package_json() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        let docs = dir.path().join("docs");
        fs::create_dir_all(app.join("dist")).unwrap();
        fs::create_dir_all(docs.join("build")).unwrap();
        fs::write(app.join("package.json"), "{}").unwrap();

        let rule = MatchRule::NameInJsPackage("dist");
        assert!(rule.matches(&app.join("dist")));
        assert!(!MatchRule::NameInJsPackage("build").matches(&docs.join("build")));
    }

    #[test]
    fn name_in_js_package_skips_native_trees() {
        let dir = tempdir().unwrap();
        let build = dir.path().join("android").join("app").join("build");
        fs::create_dir_all(&build).unwrap();
        fs::write(
            dir.path().join("android").join("app").join("package.json"),
            "{}",
        )
        .unwrap();

        assert!(!MatchRule::NameInJsPackage("build").matches(&build));
        assert!(MatchRule::PathSuffix(&["android", "app", "build"]).matches(&build));
    }

    #[test]
    fn name_with_sibling_next_export() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        fs::create_dir_all(app.join("out")).unwrap();
        fs::write(app.join("next.config.ts"), "export default {}").unwrap();

        let rule = MatchRule::NameWithSibling("out", &[".next", "next.config.*"]);
        assert!(rule.matches(&app.join("out")));
        assert!(!rule.matches(&PathBuf::from("docs/out")));
    }

    #[test]
    fn git_is_protected_even_if_named_as_a_target() {
        assert!(is_protected(OsStr::new(".git")));
        assert!(!is_protected(OsStr::new("node_modules")));
    }
}
