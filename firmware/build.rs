use std::env;
use std::fs;
use std::path::Path;

fn main() {
    embuild::espidf::sysenv::output();

    let env_file = Path::new(".env");
    let count_raw = env::var("ENCODER_COUNT").ok().or_else(|| {
        if env_file.exists() {
            dotenvy::from_path_iter(env_file)
                .ok()?
                .filter_map(Result::ok)
                .find(|(key, _)| key == "ENCODER_COUNT")
                .map(|(_, value)| value)
        } else {
            None
        }
    });

    let count: usize = count_raw.as_deref().unwrap_or("8").parse().unwrap_or(8);

    if count == 0 || count > 8 {
        panic!("ENCODER_COUNT must be between 1 and 8, got {}", count);
    }

    println!("cargo:rustc-env=ENCODER_COUNT={}", count);
    println!("cargo:rerun-if-changed=.env");

    let out_dir = env::var("OUT_DIR").unwrap();
    let config_path = Path::new(&out_dir).join("encoder_count.rs");
    fs::write(
        &config_path,
        format!("pub const ENCODER_COUNT: usize = {};\n", count),
    )
    .unwrap();
}
