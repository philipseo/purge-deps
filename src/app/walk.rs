use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{WalkBuilder, WalkState};

use super::config::Config;
use super::error::Error;
use super::match_rule::{self, MatchRule};
use super::purge;
use super::Report;

/// A matched file or folder, collected before any delete happens.
struct Candidate {
    path: PathBuf,
    is_dir: bool,
}

/// Recursively delete (or dry-run) target files and folders under `config.path`.
///
/// Example: with defaults, `./packages/web/node_modules` is removed and
/// `./src/node_modules` is not, because `src` is in `ignore`.
pub fn purge(config: &Config) -> Result<Report, Error> {
    let gitignore = load_gitignore(config)?;
    let mut candidates = collect_candidates(config, &gitignore)?;
    prune_children(&mut candidates);

    let mut report = Report::default();
    for candidate in candidates {
        if config.dry_run {
            if candidate.is_dir {
                println!("Would delete folder: {:?}", candidate.path);
            } else {
                println!("Would delete file: {:?}", candidate.path);
            }
        } else if candidate.is_dir {
            purge::delete_dir(&candidate.path)?;
        } else {
            purge::delete_file(&candidate.path)?;
        }
        report.deleted.push(candidate.path);
    }

    Ok(report)
}

/// Load `{config.path}/.gitignore` as glob rules. Missing files print a notice.
///
/// Example: `coverage/` skips that directory; `node_modules/` still deletes
/// because it is a target.
fn load_gitignore(config: &Config) -> Result<Gitignore, Error> {
    if !config.use_gitignore {
        return Ok(Gitignore::empty());
    }

    let gitignore_path = config.path.join(".gitignore");
    if !gitignore_path.is_file() {
        return Ok(Gitignore::empty());
    }

    let mut builder = GitignoreBuilder::new(&config.path);
    if let Some(err) = builder.add(&gitignore_path) {
        return Err(Error::Walk(err.to_string()));
    }

    builder.build().map_err(|err| Error::Walk(err.to_string()))
}

/// Parallel walk: skip ignore dirs, collect targets, skip gitignored non-targets.
fn collect_candidates(config: &Config, gitignore: &Gitignore) -> Result<Vec<Candidate>, Error> {
    let targets = Arc::new(config.exact_name_set());
    let ignore = Arc::new(config.ignore_set());
    let path_rules = Arc::new(config.path_rules());
    let gitignore = gitignore.clone();
    let found = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));

    WalkBuilder::new(&config.path)
        .hidden(false)
        .standard_filters(false)
        .follow_links(false)
        .build_parallel()
        .run(|| {
            let targets = Arc::clone(&targets);
            let ignore = Arc::clone(&ignore);
            let path_rules = Arc::clone(&path_rules);
            let gitignore = gitignore.clone();
            let found = Arc::clone(&found);
            let errors = Arc::clone(&errors);

            Box::new(move |entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        errors.lock().expect("walk errors").push(err.to_string());
                        return WalkState::Continue;
                    }
                };

                // Root of the walk is the search path itself, not a candidate.
                if entry.depth() == 0 {
                    return WalkState::Continue;
                }

                classify_entry(&entry, &targets, &ignore, &path_rules, &gitignore, &found)
            })
        });

    let errors = errors.lock().expect("walk errors");
    if let Some(message) = errors.first() {
        return Err(Error::Walk(message.clone()));
    }

    Ok(Arc::into_inner(found)
        .expect("walk finished")
        .into_inner()
        .expect("walk candidates"))
}

/// Decide Skip / collect / continue for one directory entry.
///
/// Example: `src` → Skip; `node_modules` → collect + Skip; `coverage/` in
/// gitignore → Skip; other directories → Continue.
fn classify_entry(
    entry: &ignore::DirEntry,
    targets: &HashSet<OsString>,
    ignore: &HashSet<OsString>,
    path_rules: &[MatchRule],
    gitignore: &Gitignore,
    found: &Mutex<Vec<Candidate>>,
) -> WalkState {
    let file_name = entry.file_name();
    let file_type = entry.file_type();
    let is_symlink = file_type.is_some_and(|file_type| file_type.is_symlink());
    let is_dir = file_type.is_some_and(|file_type| file_type.is_dir()) && !is_symlink;
    let is_file = file_type.is_some_and(|file_type| file_type.is_file());

    // Ignore wins over targets: a name in both lists is skipped, not deleted.
    if ignore.contains(file_name) {
        return WalkState::Skip;
    }

    // `.git` is never a candidate, even if a rule or `-t` names it.
    if match_rule::is_protected(file_name) {
        return WalkState::Skip;
    }

    // Exact basename HashSet first; path-aware rules only on a miss.
    let matched =
        targets.contains(file_name) || path_rules.iter().any(|rule| rule.matches(entry.path()));
    if matched {
        if is_file || is_dir || is_symlink {
            found.lock().expect("walk candidates").push(Candidate {
                path: entry.path().to_path_buf(),
                is_dir,
            });
        }
        // Do not list the inside of a target directory (e.g. `node_modules/**`).
        return WalkState::Skip;
    }

    // Gitignore glob skip, but never for names already handled as targets above.
    if gitignore.matched(entry.path(), is_dir).is_ignore() {
        return WalkState::Skip;
    }

    WalkState::Continue
}

/// Drop candidates that live under another candidate that will already be deleted.
///
/// Example: keep `app/node_modules`, drop `app/node_modules/left-over` if both appeared.
fn prune_children(candidates: &mut Vec<Candidate>) {
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let mut kept: Vec<Candidate> = Vec::new();
    for candidate in candidates.drain(..) {
        let nested = kept.iter().any(|parent| {
            candidate
                .path
                .strip_prefix(&parent.path)
                .is_ok_and(|rest| !rest.as_os_str().is_empty())
        });
        if !nested {
            kept.push(candidate);
        }
    }
    *candidates = kept;
}

#[cfg(test)]
mod tests {
    use super::super::match_rule::MatchRule;
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn config_without_gitignore(path: &Path) -> Config {
        let mut config = Config::new(path);
        config.use_gitignore = false;
        config
    }

    #[test]
    fn dry_run_does_not_delete() {
        let dir = tempdir().unwrap();
        let node_modules = dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        fs::write(node_modules.join("pkg"), "x").unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.dry_run = true;
        let report = purge(&config).unwrap();

        assert_eq!(report.deleted.len(), 1);
        assert!(node_modules.exists());
    }

    #[test]
    fn gitignore_globs_skip_non_targets_but_keep_targets() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".gitignore"),
            "coverage/\n*.log\nnode_modules/\n",
        )
        .unwrap();

        let coverage = dir.path().join("coverage");
        fs::create_dir(&coverage).unwrap();
        fs::write(coverage.join("report"), "x").unwrap();
        fs::write(dir.path().join("debug.log"), "x").unwrap();

        let node_modules = dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        fs::write(node_modules.join("pkg"), "x").unwrap();

        let report = purge(&Config::new(dir.path())).unwrap();

        assert!(coverage.exists());
        assert!(dir.path().join("debug.log").exists());
        assert!(!node_modules.exists());
        assert!(report
            .deleted
            .iter()
            .any(|path| path.file_name() == Some(std::ffi::OsStr::new("node_modules"))));
    }

    #[test]
    fn prune_drops_nested_paths() {
        let mut candidates = vec![
            Candidate {
                path: PathBuf::from("/app/node_modules"),
                is_dir: true,
            },
            Candidate {
                path: PathBuf::from("/app/node_modules/left-over"),
                is_dir: false,
            },
            Candidate {
                path: PathBuf::from("/app/package-lock.json"),
                is_dir: false,
            },
        ];
        prune_children(&mut candidates);
        assert_eq!(candidates.len(), 2);
        assert!(candidates
            .iter()
            .any(|c| c.path == Path::new("/app/node_modules")));
        assert!(candidates
            .iter()
            .any(|c| c.path == Path::new("/app/package-lock.json")));
    }

    #[test]
    fn default_config_does_not_delete_android_build() {
        let dir = tempdir().unwrap();
        let build = dir.path().join("android").join("app").join("build");
        fs::create_dir_all(&build).unwrap();
        fs::write(build.join("out"), "x").unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(build.exists());
    }

    #[test]
    fn default_deletes_next_export_out() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        fs::create_dir_all(app.join("out")).unwrap();
        fs::write(app.join("next.config.ts"), "export default {}").unwrap();
        fs::write(app.join("out").join("index.html"), "x").unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(!app.join("out").exists());
        assert!(app.join("next.config.ts").exists());
    }

    #[test]
    fn extra_path_suffix_rule_deletes_android_build() {
        let dir = tempdir().unwrap();
        let build = dir.path().join("android").join("app").join("build");
        fs::create_dir_all(&build).unwrap();
        fs::write(build.join("out"), "x").unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.extra_rules = vec![MatchRule::PathSuffix(&["android", "app", "build"])];
        purge(&config).unwrap();

        assert!(!build.exists());
    }

    #[test]
    fn default_deletes_js_package_dist_but_not_docs_dist() {
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        let docs = dir.path().join("docs");
        fs::create_dir_all(app.join("dist")).unwrap();
        fs::write(app.join("package.json"), "{}").unwrap();
        fs::create_dir_all(docs.join("dist")).unwrap();
        fs::write(docs.join("dist").join("index.html"), "x").unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(!app.join("dist").exists());
        assert!(docs.join("dist").exists());
    }

    #[test]
    fn default_deletes_vercel_output_not_whole_vercel() {
        let dir = tempdir().unwrap();
        let vercel = dir.path().join(".vercel");
        fs::create_dir_all(vercel.join("output")).unwrap();
        fs::write(vercel.join("project.json"), "{}").unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(!vercel.join("output").exists());
        assert!(vercel.join("project.json").exists());
    }

    #[test]
    fn default_deletes_turbo_cache() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".turbo")).unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(!dir.path().join(".turbo").exists());
    }

    #[test]
    fn protected_git_is_not_deleted_even_as_a_target() {
        let dir = tempdir().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir(&git).unwrap();
        fs::write(git.join("HEAD"), "ref").unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.ignore.clear();
        config.extra_rules = vec![MatchRule::ExactName(".git")];
        purge(&config).unwrap();

        assert!(git.exists());
    }

    #[cfg(unix)]
    #[test]
    fn deletes_symlink_without_following() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let real = dir.path().join("real_modules");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("pkg"), "keep").unwrap();
        let link = dir.path().join("node_modules");
        symlink(&real, &link).unwrap();

        purge(&config_without_gitignore(dir.path())).unwrap();

        assert!(link.symlink_metadata().is_err());
        assert!(real.exists());
        assert!(real.join("pkg").exists());
    }
}
