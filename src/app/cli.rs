use std::env;
use std::path::PathBuf;

use clap::Parser;

use super::config::{parse_list, Config};
use super::presets::Preset;

/// Flag names accepted without `--` (`path` → `--path`).
const BARE_FLAGS: &[&str] = &[
    "help",
    "path",
    "targets",
    "extends",
    "ignore",
    "gitignore",
    "preset",
];

/// Command-line arguments. Dash-less aliases are normalized before clap sees them.
///
/// Example: `purge-deps -p ./apps -e dist` → path `./apps`, targets = defaults + `dist`.
#[derive(Debug, Parser)]
#[command(
    name = "purge-deps",
    version,
    about = "Delete JavaScript dependency leftovers and frontend build caches",
    long_about = None,
    override_usage = "purge-deps [OPTIONS]",
    after_help = "Presets: js (lockfiles + node_modules), frontend (Next/Nuxt/Vite caches, .turbo, JS-package dist/build), mobile (Expo/RN leftovers; opt-in).\nDefault: js,frontend. .git is never deleted."
)]
pub struct Args {
    /// Specify the path to delete files and folders
    #[arg(short = 'p', long = "path", default_value = ".", value_name = "PATH")]
    pub path: PathBuf,

    /// Presets to enable (comma-separated). mobile is opt-in
    #[arg(
        long = "preset",
        value_delimiter = ',',
        default_value = "js,frontend",
        value_name = "PRESETS"
    )]
    pub presets: Vec<Preset>,

    /// Replace the targets to delete (comma-separated). Ignores presets
    #[arg(
        short = 't',
        long = "targets",
        value_name = "TARGETS",
        conflicts_with = "extends"
    )]
    pub targets: Option<String>,

    /// Add to the targets to delete (comma-separated)
    #[arg(
        short = 'e',
        long = "extends",
        value_name = "TARGETS",
        conflicts_with = "targets"
    )]
    pub extends: Option<String>,

    /// Add folders to the default ignore list (comma-separated)
    #[arg(
        short = 'i',
        long = "ignore",
        value_name = "FOLDERS",
        conflicts_with = "replace_ignore"
    )]
    pub ignore: Option<String>,

    /// Replace the ignore list entirely (comma-separated)
    #[arg(
        long = "replace-ignore",
        value_name = "FOLDERS",
        conflicts_with = "ignore"
    )]
    pub replace_ignore: Option<String>,

    /// Enable or disable reading from .gitignore
    #[arg(
        short = 'g',
        long = "gitignore",
        visible_alias = "gi",
        default_value = "true",
        default_missing_value = "true",
        num_args = 0..=1,
        value_name = "true|false"
    )]
    pub gitignore: String,

    /// List matches without deleting them
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

/// Map bare flag names (`path`, `-gi`) to clap long options.
///
/// Example: `["purge-deps", "path", "./apps", "-gi", "false"]`
/// → `["purge-deps", "--path", "./apps", "--gitignore", "false"]`.
pub fn normalize_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|arg| {
            let arg = arg.as_ref();
            if BARE_FLAGS.contains(&arg) {
                format!("--{arg}")
            } else if arg == "-gi" {
                // clap short flags are one character; `-gi` is `--gitignore`.
                "--gitignore".to_string()
            } else {
                arg.to_string()
            }
        })
        .collect()
}

/// Parse process argv after mapping bare flag names.
///
/// Example: `purge-deps path .` is accepted the same as `purge-deps --path .`.
pub fn parse() -> Args {
    Args::parse_from(normalize_args(env::args()))
}

impl From<Args> for Config {
    /// Build a [`Config`] from parsed CLI flags.
    ///
    /// Example: `--preset js` keeps lockfiles only; `-i vendor` extends ignore;
    /// `-t foo` replaces presets entirely.
    fn from(args: Args) -> Self {
        let mut config = Config {
            path: args.path,
            targets: Vec::new(),
            ignore: Config::default_ignore(),
            use_gitignore: args.gitignore.to_lowercase() != "false",
            dry_run: args.dry_run,
            presets: Vec::new(),
            extra_rules: Vec::new(),
        };

        if let Some(targets) = args.targets {
            // `-t` replaces presets: exact names only, no path-aware rules.
            config.targets = parse_list(&targets);
        } else {
            config.apply_presets(&args.presets);
            if let Some(extends) = args.extends {
                for name in parse_list(&extends) {
                    if !config.targets.contains(&name) {
                        config.targets.push(name);
                    }
                }
            }
        }

        if let Some(replace_ignore) = args.replace_ignore {
            config.ignore = parse_list(&replace_ignore);
        } else if let Some(ignore) = args.ignore {
            config.extend_ignore(parse_list(&ignore));
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn normalize_dashless_and_gi() {
        let args = normalize_args(["purge-deps", "path", "./apps", "-gi", "false"]);
        assert_eq!(
            args,
            vec!["purge-deps", "--path", "./apps", "--gitignore", "false"]
        );
    }

    #[test]
    fn targets_replaces_defaults() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "-t", "foo, bar"])).unwrap();
        let config = Config::from(args);
        assert_eq!(config.targets, vec!["foo", "bar"]);
        assert!(config.extra_rules.is_empty());
        assert!(!config.targets.contains(&".next".to_string()));
    }

    #[test]
    fn extends_appends_to_defaults() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "-e", "custom"])).unwrap();
        let config = Config::from(args);
        assert!(config.targets.contains(&"node_modules".to_string()));
        assert!(config.targets.contains(&"custom".to_string()));
    }

    #[test]
    fn targets_conflicts_with_extends() {
        let result = Args::try_parse_from(normalize_args(["purge-deps", "-t", "a", "-e", "b"]));
        assert!(result.is_err());
    }

    #[test]
    fn ignore_extends_defaults() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "-i", "vendor"])).unwrap();
        let config = Config::from(args);
        assert!(config.ignore.contains(&"vendor".to_string()));
        assert!(config.ignore.contains(&".git".to_string()));
        assert!(config.ignore.contains(&"src".to_string()));
    }

    #[test]
    fn replace_ignore_replaces_defaults() {
        let args =
            Args::try_parse_from(normalize_args(["purge-deps", "--replace-ignore", "vendor"]))
                .unwrap();
        let config = Config::from(args);
        assert_eq!(config.ignore, vec!["vendor"]);
    }

    #[test]
    fn gitignore_false_disables() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "-gi", "false"])).unwrap();
        let config = Config::from(args);
        assert!(!config.use_gitignore);
    }

    #[test]
    fn dry_run_flag() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "--dry-run"])).unwrap();
        let config = Config::from(args);
        assert!(config.dry_run);
    }

    #[test]
    fn default_presets_are_js_frontend() {
        let args = Args::try_parse_from(normalize_args(["purge-deps"])).unwrap();
        let config = Config::from(args);
        assert_eq!(config.presets, vec![Preset::Js, Preset::Frontend]);
        assert!(config.targets.contains(&".next".to_string()));
        assert!(!config.targets.contains(&".expo".to_string()));
    }

    #[test]
    fn preset_js_drops_frontend() {
        let args = Args::try_parse_from(normalize_args(["purge-deps", "--preset", "js"])).unwrap();
        let config = Config::from(args);
        assert_eq!(config.presets, vec![Preset::Js]);
        assert!(config.targets.contains(&"node_modules".to_string()));
        assert!(!config.targets.contains(&".next".to_string()));
        assert!(config.extra_rules.is_empty());
    }

    #[test]
    fn preset_mobile_is_opt_in() {
        let args = Args::try_parse_from(normalize_args([
            "purge-deps",
            "--preset",
            "js,frontend,mobile",
        ]))
        .unwrap();
        let config = Config::from(args);
        assert!(config.targets.contains(&".expo".to_string()));
        assert!(config.targets.contains(&".next".to_string()));
    }

    #[test]
    fn unknown_preset_is_rejected() {
        let result = Args::try_parse_from(normalize_args(["purge-deps", "--preset", "python"]));
        assert!(result.is_err());
    }

    #[test]
    fn preset_mobile_only_drops_js_and_frontend() {
        let args =
            Args::try_parse_from(normalize_args(["purge-deps", "--preset", "mobile"])).unwrap();
        let config = Config::from(args);
        assert_eq!(config.presets, vec![Preset::Mobile]);
        assert!(config.targets.contains(&".expo".to_string()));
        assert!(!config.targets.contains(&"node_modules".to_string()));
        assert!(!config.targets.contains(&".next".to_string()));
    }
}
