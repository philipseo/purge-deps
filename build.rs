fn main() {
    println!("cargo:rerun-if-changed=package.json");
    let package_json = std::fs::read_to_string("package.json").expect("package.json");
    let version = parse_npm_version(&package_json).expect("version field in package.json");
    println!("cargo:rustc-env=PURGE_DEPS_VERSION={version}");
}

fn parse_npm_version(package_json: &str) -> Option<String> {
    for line in package_json.lines() {
        let line = line.trim().trim_end_matches(',');
        let Some(rest) = line.strip_prefix("\"version\"") else {
            continue;
        };
        let value = rest.trim().trim_start_matches(':').trim().trim_matches('"');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}
