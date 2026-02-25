use std::path::PathBuf;
use std::process::Command;

fn command_exists(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_command(program: &str, args: &[&str], cwd: &PathBuf) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command `{}` failed with status {}",
            std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" "),
            status
        ))
    }
}

fn build_frontend(frontend_dir: &PathBuf) -> Result<(), String> {
    if command_exists("bun") {
        run_command("bun", &["install", "--frozen-lockfile"], frontend_dir)?;
        run_command("bun", &["run", "build:embed"], frontend_dir)?;
        return Ok(());
    }

    if command_exists("npm") {
        run_command("npm", &["install", "--no-audit", "--no-fund"], frontend_dir)?;
        run_command("npm", &["run", "build:embed"], frontend_dir)?;
        return Ok(());
    }

    Err(
        "neither `bun` nor `npm` is available to build frontend assets"
            .to_string(),
    )
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let frontend_dir = manifest_dir.join("../../frontend");
    let dist_dir = manifest_dir.join("../../frontend/dist");
    let index_html = dist_dir.join("index.html");

    println!("cargo:rerun-if-changed=../../frontend/src");
    println!("cargo:rerun-if-changed=../../frontend/public");
    println!("cargo:rerun-if-changed=../../frontend/package.json");
    println!("cargo:rerun-if-changed=../../frontend/bun.lock");
    println!("cargo:rerun-if-changed=../../frontend/dist");

    if !index_html.exists() {
        println!(
            "cargo:warning=frontend/dist missing, building embedded dashboard assets"
        );

        if let Err(err) = build_frontend(&frontend_dir) {
            if std::env::var("ST_ALLOW_MISSING_FRONTEND")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
            {
                std::fs::create_dir_all(&dist_dir)
                    .expect("failed to create frontend/dist directory");
                println!(
                    "cargo:warning=frontend build skipped (ST_ALLOW_MISSING_FRONTEND), assets will not be embedded: {err}"
                );
            } else {
                panic!(
                    "frontend/dist is required for RustEmbed and could not be built automatically: {err}. \
                     Install bun (preferred) or npm, or set ST_ALLOW_MISSING_FRONTEND=1 to allow a build without embedded dashboard assets."
                );
            }
        }
    }
}
