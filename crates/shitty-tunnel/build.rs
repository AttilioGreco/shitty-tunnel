use std::path::PathBuf;
use std::process::Command;

fn command_exists(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn first_available_command(programs: &[&str]) -> Option<String> {
    programs
        .iter()
        .find(|program| command_exists(program))
        .map(|program| (*program).to_string())
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
    if let Some(bun) = first_available_command(&["bun", "bun.cmd", "bun.exe"]) {
        run_command(&bun, &["install", "--frozen-lockfile"], frontend_dir)?;
        run_command(&bun, &["run", "build:embed"], frontend_dir)?;
        return Ok(());
    }

    if let Some(npm) = first_available_command(&["npm", "npm.cmd", "npm.exe"]) {
        run_command(
            &npm,
            &["install", "--no-audit", "--no-fund"],
            frontend_dir,
        )?;
        run_command(&npm, &["run", "build:embed"], frontend_dir)?;
        return Ok(());
    }

    Err(
        "neither `bun` nor `npm` is available to build frontend assets"
            .to_string(),
    )
}

fn write_placeholder_frontend(dist_dir: &PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(dist_dir)
                .map_err(|err| format!("failed to create frontend/dist: {err}"))?;

        let placeholder = r#"<!doctype html>
<html lang=\"en\">
    <head>
        <meta charset=\"utf-8\" />
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
        <title>shittyTunnel Dashboard</title>
    </head>
    <body style=\"font-family: sans-serif; padding: 1rem\">
        <h1>Dashboard assets unavailable</h1>
        <p>This build was produced without frontend toolchain (bun/npm).</p>
    </body>
</html>
"#;

        std::fs::write(dist_dir.join("index.html"), placeholder)
                .map_err(|err| format!("failed to write placeholder index.html: {err}"))?;

        Ok(())
}

fn emit_build_info() {
    // Git commit SHA
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Git dirty flag
    let git_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let sha_display = if git_dirty {
        format!("{git_sha}-dirty")
    } else {
        git_sha
    };

    // Build timestamp (UTC)
    let build_date = chrono_lite_utc_now();

    println!("cargo:rustc-env=ST_GIT_SHA={sha_display}");
    println!("cargo:rustc-env=ST_BUILD_DATE={build_date}");
}

/// Minimal UTC timestamp without pulling in chrono.
fn chrono_lite_utc_now() -> String {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%d %H:%M:%S UTC"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

fn main() {
    emit_build_info();

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
            let strict_frontend = std::env::var("ST_REQUIRE_FRONTEND")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or_else(|_| std::env::var("CI").is_ok());

            if strict_frontend {
                panic!(
                    "frontend/dist is required for RustEmbed and could not be built automatically: {err}. \
                     Install bun (preferred) or npm, or unset ST_REQUIRE_FRONTEND to allow placeholder assets."
                );
            }

            if let Err(placeholder_err) = write_placeholder_frontend(&dist_dir) {
                panic!(
                    "frontend build failed: {err}; additionally failed to create placeholder assets: {placeholder_err}"
                );
            }

            println!(
                "cargo:warning=frontend build skipped ({err}); embedded placeholder dashboard instead"
            );
        }
    }
}
