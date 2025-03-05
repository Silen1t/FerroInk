use ::{ std::{ env, fs, io, path::{Path, PathBuf} }, winresource::WindowsResource };

fn main() -> io::Result<()> {
    // Don't add nay code above this code block because you will be facing issues like
    // environment variable `SLINT_INCLUDE_GENERATED` not defined at compile time use `std::env::var(\"SLINT_INCLUDE_GENERATED\")` to read the variable at run time
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");

    // For windows only
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("ui/assets/images/FerroInkAppIcon.ico")
            .compile()?;
    }
    
    // Get the project root (where Cargo.toml is located)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // The folder you want to copy (adjust the folder name if different)
    let source_folder = Path::new(manifest_dir).join("Inkscape");

    // Get the OUT_DIR environment variable
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut target_dir = PathBuf::from(out_dir);

    // The OUT_DIR is something like:
    //   target/release/build/<crate>/out
    // We want to get to target/release so we pop three components:
    for _ in 0..3 {
        target_dir.pop();
    }

    // Now target_dir should be target/release.
    let dest_folder = target_dir.join("Inkscape");

    // Copy the entire folder recursively
    if source_folder.exists() {
        copy_dir_all(&source_folder, &dest_folder).expect("Failed to copy Inkscape folder");
        println!("cargo:rerun-if-changed={}", source_folder.display());
        println!("Copied Inkscape folder to {}", dest_folder.display());
    } else {
        panic!("Source Inkscape folder not found: {}", source_folder.display());
    }

    Ok(())
}

// A helper function to recursively copy a directory.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
