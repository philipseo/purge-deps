use std::env;
use std::fs;
use std::path::Path;
use std::sync::RwLock;
use once_cell::sync::Lazy;    

static TARGETS: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(vec![
    "node_modules".to_string(),
    "pnpm-lock.yaml".to_string(),
    "yarn.lock".to_string(),
    "package-lock.json".to_string(),
]));

fn print_help() {
    println!("Usage: puge-deps [options]");
    println!("Options:");
    println!("  -h or help                  Show this help message.");
    println!("  -p or path <path>           Specify the path to delete files and folders.");
    println!("  -e or extends <targets>     Add to the list of targets to delete.");
    println!("  -o or overwrite <list>      Replace the list of targets to delete.");
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

fn recursive_delete_files_and_folders<P: AsRef<Path>>(
    path: P
) -> Result<(), std::io::Error> {
    let entries: fs::ReadDir = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name_osstr = entry.file_name();

        if let Some(file_name) = file_name_osstr.to_str() {
            
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
        } else {
            eprintln!("Error: Invalid file name for entry: {:?}", entry_path);
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut path = ".";
    let mut set_extends = false;
    let mut set_overwrite = false;
    
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
            "extends" | "-e" => {
                if set_overwrite {
                    eprintln!("Error: 'extends' cacnnot be used with 'overwrite'.");
                    return;
                }

                if i + 1 < args.len() {
                    let new_targets = parse_targets(&args[i + 1]);
                    TARGETS.write().unwrap().extend(new_targets);
                    set_extends = true;
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a list after 'extends'.");
                }
            }
            "overwrite" | "-o" => {
                if set_extends {
                    eprintln!("Error: 'overwrite' cannot be used with 'extends'.");
                    return;
                }

                if i + 1 < args.len() {
                    let new_targets = parse_targets(&args[i + 1]);
                    *TARGETS.write().unwrap() = new_targets;
                    set_overwrite = true;
                    i += 1;
                } else {
                    eprintln!("Error: You must specify a list after 'overwrite'.");
                    return;
                }
            }
            _ => {
                eprintln!("Error: Unknown option {}. Use help or -h for usage information.", args[i]);
                return;
            }
        }
        i += 1;
    }

    println!("Path: {}", path);
    println!("Targets: {:?}", *TARGETS.read().unwrap());

    if let Err(err) = recursive_delete_files_and_folders(path) {
        eprintln!("Error: {}", err);
    }
}
