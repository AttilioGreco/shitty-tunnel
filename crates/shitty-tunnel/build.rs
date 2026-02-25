use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist_dir = manifest_dir.join("../../frontend/dist");

    // RustEmbed requires the folder to exist at compile time.
    // Create an empty one when the frontend hasn't been built yet so that
    // `cargo build` always succeeds. The embedded assets will simply be empty
    // in that case and the server will return 404 for dashboard requests.
    if !dist_dir.exists() {
        std::fs::create_dir_all(&dist_dir)
            .expect("failed to create frontend/dist directory");
        println!(
            "cargo:warning=frontend/dist does not exist — created an empty directory. \
             Run `npm run build:embed` inside frontend/ to build the dashboard assets."
        );
    }

    // Re-run this script when the frontend source or dist contents change.
    println!("cargo:rerun-if-changed=../../frontend/src");
    println!("cargo:rerun-if-changed=../../frontend/dist");
}
