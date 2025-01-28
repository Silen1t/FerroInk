use {
    std::{env, io},
    winresource::WindowsResource,
};

fn main() -> io::Result<()> {
    // Don't add nay code above this code block because you will be facing issues like
    // environment variable `SLINT_INCLUDE_GENERATED` not defined at compile time use `std::env::var(\"SLINT_INCLUDE_GENERATED\")` to read the variable at run time
    slint_build::compile("ui/app-window.slint").expect("Slint build failed");

    // For windows only
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("ui/assets/images/FormatifyAppIcon.ico")
            .compile()?;
    }

    Ok(())
}
