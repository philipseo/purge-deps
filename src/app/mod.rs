//! Application logic: CLI, config, walk, and delete.
//!
//! `src/main.rs` is a thin binary. It converts argv into [`Config`] and calls [`run`].

pub mod cli;
pub mod config;
pub mod error;
pub mod match_rule;
pub mod presets;
mod purge;
mod walk;

pub use config::Config;
pub use error::Error;

/// Paths that were deleted (or would be deleted in dry-run) during a run.
///
/// Example: after removing `./app/node_modules`, `deleted` contains that path.
#[derive(Debug, Default, Clone)]
pub struct Report {
    pub deleted: Vec<std::path::PathBuf>,
}

/// Print the resolved config, then collect and delete matching files.
///
/// Example: `run(Config::new("."))` uses `js` + `frontend` under `.`.
/// Gitignore globs are read from `{path}/.gitignore`.
pub fn run(config: Config) -> Result<Report, Error> {
    let preset_names: Vec<_> = config
        .presets
        .iter()
        .map(|preset| preset.as_str())
        .collect();
    println!("PATH: {}", config.path.display());
    println!("PRESETS: {preset_names:?}");
    println!("TARGETS: {:?}", config.targets);
    println!("IGNORE: {:?}", config.ignore);
    println!("USE_GITIGNORE: {:?}", config.use_gitignore);
    println!("DRY_RUN: {:?}", config.dry_run);

    walk::purge(&config)
}

#[cfg(test)]
mod tests {
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
    fn deletes_default_targets() {
        let dir = tempdir().unwrap();
        let node_modules = dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        fs::write(node_modules.join("pkg"), "x").unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        fs::write(dir.path().join("keep.txt"), "ok").unwrap();

        run(config_without_gitignore(dir.path())).unwrap();

        assert!(!dir.path().join("node_modules").exists());
        assert!(!dir.path().join("package-lock.json").exists());
        assert!(dir.path().join("keep.txt").exists());
    }

    #[test]
    fn skips_targets_inside_ignored_src() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("src").join("node_modules");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("pkg"), "x").unwrap();

        run(config_without_gitignore(dir.path())).unwrap();

        assert!(nested.exists());
    }

    #[test]
    fn dry_run_keeps_files() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.dry_run = true;
        run(config).unwrap();

        assert!(dir.path().join("node_modules").exists());
    }

    #[test]
    fn default_deletes_next_but_not_expo() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".next")).unwrap();
        fs::create_dir(dir.path().join(".expo")).unwrap();
        fs::create_dir_all(dir.path().join("ios").join("Pods")).unwrap();

        run(config_without_gitignore(dir.path())).unwrap();

        assert!(!dir.path().join(".next").exists());
        assert!(dir.path().join(".expo").exists());
        assert!(dir.path().join("ios").join("Pods").exists());
    }

    #[test]
    fn js_preset_does_not_delete_next() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".next")).unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.apply_presets(&[crate::app::presets::Preset::Js]);
        run(config).unwrap();

        assert!(dir.path().join(".next").exists());
        assert!(!dir.path().join("node_modules").exists());
    }

    #[test]
    fn mobile_preset_deletes_expo_and_pods() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".expo")).unwrap();
        fs::create_dir_all(dir.path().join("ios").join("Pods")).unwrap();

        let mut config = config_without_gitignore(dir.path());
        config.apply_presets(&[
            crate::app::presets::Preset::Js,
            crate::app::presets::Preset::Frontend,
            crate::app::presets::Preset::Mobile,
        ]);
        run(config).unwrap();

        assert!(!dir.path().join(".expo").exists());
        assert!(!dir.path().join("ios").join("Pods").exists());
    }
}
