use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::RwLock;
use once_cell::sync::Lazy;    

static TARGETS: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(vec![
    "node_modules".to_string(),
    "pnpm-lock.yaml".to_string(),
    "yarn.lock".to_string(),
    "package-lock.json".to_string(),
]));
static IGNORE: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(vec![
    ".changeset".to_string(),
    ".husky".to_string(),
    ".git".to_string(),
    ".github".to_string(),
    "src".to_string(),
]));
static USE_GITIGNORE: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(true));

fn print_help() {
    println!("Usage: puge-deps [options]");
    println!("Options:");
    println!("  -h or help                      Show this help message.");
    println!("  -p or path <path>               Specify the path to delete files and folders.");
    println!("  -t or targets <targets>         Replace the targets to delete.");
    println!("  -e or extends <targets>         Add to the targets to delete.");
    println!("  -i or ignore <folders>          Specify folders to ignore.");
    println!("  -g or gitignore <true|false>    Enable or disable reading from .gitignore.")
}

fn parse_targets(input: &str) -> Vec<String> {
    input.split(',')
        .filter_map(|s| {
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                Some(trimmed)
            } else {
                None
            }
        })
        .collect()
}

fn should_ignore(folder_name: &str) -> bool {
    IGNORE.read().unwrap().contains(&folder_name.to_string())
}

fn load_gitignore<P: AsRef<Path>>(path: P) -> Result<(), io::Error> {
    let file = fs::File::open(path);

    match file {
        Ok(reader) => {
            let reader = io::BufReader::new(reader);

            for line in reader.lines() {
                let line = line?;
                let trimmed = line.trim();
                
                if !trimmed.is_empty() && !trimmed.starts_with('#') && !TARGETS.read().unwrap().contains(&trimmed.to_string()) {
                    IGNORE.write().unwrap().push(trimmed.to_string());
                }
            }
        }
        Err(_) => {
            println!(".gitignore file not found");
        }
    }
    Ok(())
}

fn recursive_delete_files_and_folders<P: AsRef<Path>>(
    path: P
) -> Result<(), std::io::Error> {
    let entries: fs::ReadDir = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if should_ignore(&file_name) {
            continue;
        }

        let is_file = entry_path.is_file();
        let is_dir = entry_path.is_dir();

        if TARGETS.read().unwrap().contains(&file_name.to_string()) {
            if is_file {
                println!("Deleting file: {:?}", entry_path);
                fs::remove_file(&entry_path).map_err(|err| {
                    eprintln!("Error deleting file {:?}: {}", entry_path, err);
                    err
                })?;
            } else if is_dir {
                println!("Deleting folder: {:?}", entry_path);
                fs::remove_dir_all(&entry_path).map_err(|err| {
                    eprintln!("Error Deleting folder {:?}: {}", entry_path, err);
                    err
                })?;
            }
        } else {
            if is_dir {
                recursive_delete_files_and_folders(&entry_path)?;
            }
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut path = ".";
    let mut set_extends = false;
    let mut set_targets = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "help" | "-h" => {
                print_help();
                return;
            }
            "path" | "-p" => {
                if i + 1 < args.len() {
                    path = &args[i + 1];
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a path after 'path'.");
                    return;
                }
            }
            "targets" | "-t" => {
                if set_extends {
                    eprintln!("Error: 'targets' cannot be used with 'extends'.");
                    return;
                }

                if i + 1 < args.len() {
                    let new_targets = parse_targets(&args[i + 1]);
                    *TARGETS.write().unwrap() = new_targets;
                    set_targets = true;
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a targets after 'targets'.");
                    return;
                }
            }
            "extends" | "-e" => {
                if set_targets {
                    eprintln!("Error: 'extends' cacnnot be used with 'targets'.");
                    return;
                }

                if i + 1 < args.len() {
                    let new_targets = parse_targets(&args[i + 1]);
                    TARGETS.write().unwrap().extend(new_targets);
                    set_extends = true;
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a targets after 'extends'.");
                    return;
                }
            }
            "ignore" | "-i" => {
                if i + 1 < args.len() {
                    let new_ignore = parse_targets(&args[i + 1]);
                    *IGNORE.write().unwrap() = new_ignore;
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a list after 'ignore'.");
                    return;
                }
            }
            "gitignore" | "-gi" => {
                if i + 1 < args.len() {
                    let value = args[i + 1].to_lowercase();
                    
                    if value == "false" {
                        *USE_GITIGNORE.write().unwrap() = false;
                    }
                    i += 1;
                }
            }
            _ => {
                eprintln!("Error: Unknown option {}. Use help or -h for usage information.", args[i]);
                return;
            }
        }
        i += 1;
    }

    if *USE_GITIGNORE.read().unwrap() {
        let gitignore_path = Path::new(".gitignore");
        load_gitignore(gitignore_path).ok();
    }

    println!("Path: {}", path);
    println!("Targets: {:?}", *TARGETS.read().unwrap());
    println!("Ignore>: {:?}", *IGNORE.read().unwrap());
    println!("USE_GITIGNORE: {:?}", *USE_GITIGNORE.read().unwrap());

    if let Err(err) = recursive_delete_files_and_folders(path) {
        eprintln!("Error: {}", err);
    }
}
