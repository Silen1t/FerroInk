// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rayon::prelude::*;
use rfd::AsyncFileDialog;
use slint::{
    LogicalSize, Model, ModelRc, SharedString, ToSharedString, VecModel, Weak, WindowSize,
};
use std::{collections::HashSet, env, error::Error, path::Path, process::Command, sync::Arc};

slint::include_modules!();

const START_DIRECTORY: &str = "C:\\";

fn main() -> Result<(), Box<dyn Error>> {
    let app = Arc::new(AppWindow::new()?);
    setup_window(&app);
    setup_callbacks(&app);
    app.run()?;
    Ok(())
}

/// Configure the window settings
fn setup_window(app: &Arc<AppWindow>) {
    let window_size = WindowSize::Logical(LogicalSize::new(800.0, 850.0));
    app.window().set_size(window_size);
}

fn setup_callbacks(app: &Arc<AppWindow>) {
    button_selection_image(&app);
    button_selection_image_output(&app);
    button_apply_changes(&app);
}

/// Button Selection Image
fn button_selection_image(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    app.on_ButtonSelectImageClicked(move || {
        // Spawn the async block to avoid blocking the main thread
        if let Some(app_upgrade) = app_weak.upgrade() {
            let _ = slint::spawn_local(async move {
                if let Some(files) = AsyncFileDialog::new()
                    .set_directory(START_DIRECTORY)
                    .add_filter(
                        "image",
                        &[
                            "svg", "pdf", "png"
                        ],
                    )
                    .pick_files()
                    .await
                {
                    let files_vector: Vec<SharedString> = files
                        .par_iter()
                        .map(|file| file.path().to_string_lossy().to_shared_string())
                        .collect();

                    // Wrap the VecModel in ModelRc for shared ownership
                    let converted_files_path = ModelRc::new(VecModel::from(files_vector));
                    app_upgrade.set_image_path(converted_files_path);
                }
            });
        }
    });
}

/// Button Selection Image Output Path
fn button_selection_image_output(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    app.on_ButtonSelectImageOutputClicked(move || {
        if let Some(app_upgrade) = app_weak.upgrade() {
            let _ = slint::spawn_local(async move {
                if let Some(folder) = AsyncFileDialog::new()
                    .set_directory(START_DIRECTORY)
                    .pick_folder()
                    .await
                {
                    println!("{}", folder.path().display());
                    app_upgrade
                        .set_image_output_path(folder.path().to_string_lossy().to_shared_string());
                }
            });
        }
    });
}

/// Button Selection Image
fn button_apply_changes(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    app.on_ButtonApplyChangesClicked(move |paths, output_dir, format_index| {
        if let Some(app_upgrade) = app_weak.upgrade() {
            let paths_vec: Vec<String> = (0..paths.row_count())
                .filter_map(|i| paths.row_data(i).map(|s| s.to_string()))
                .collect();
            let app_weak = app_upgrade.as_weak();
            std::thread::spawn(move || {
                process_images(paths_vec, &output_dir.to_string(), &format_index, &app_weak);
            });
        }
    });
}

// Process Images
fn process_images(
    paths: Vec<String>,
    output_dir: &String,
    format_index: &i32,
    app_weak: &Weak<AppWindow>,
) {
    // Callback
    let show_wating_screen = || {
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            app.invoke_UpdateWatingText(
                "Processing files... This may take a few moments depending on the number of files you selected. Please wait.".to_shared_string()
            );
            app.invoke_ShowWatingScreen(true);
            app.set_can_exit_wating_screen(false);
        });
    };
    show_wating_screen();

    let output_folder = output_dir.to_string();

    // Callback
    let can_exit_wating_screen = || {
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            app.invoke_UpdateWatingText(
                "File processing is complete. Click anywhere to close this message and continue."
                    .to_shared_string(),
            );

            let output = output_folder;
            app.on_ExitWatingScreen(move || {
                // the file manager when all images have finished converting
                open_file_explorer(&output);
            });
            app.set_can_exit_wating_screen(true);
        });
    };

    // Assume the binary and the Inkscape folder are in the same directory.
    let exe_path = env::current_exe().expect("Failed to get current exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let inkscape_cli = exe_dir.join(r#"Inkscape\bin\inkscape.exe"#);
    let inkscape_display_path = inkscape_cli.display().to_string();
    let inkspace_dir = inkscape_display_path.as_str();

    let format = selected_format(format_index);
    let export_type = format.as_str();

    paths.par_iter().for_each(|image_path| {
        let input_path = Path::new(image_path);

        let output_path = format!(
            "{}/{}.{}",
            &output_dir,
            input_path.file_stem().unwrap().to_str().unwrap(),
            export_type
        );

        // Input and output file paths
        let input_file = format!(r#"{}"#, input_path.display().to_shared_string().as_str());
        let output_file = format!(r#"{}"#, output_path.as_str());

        // Save image in the selected format
        convert_file(
            &input_file.as_str(),
            &output_file.as_str(),
            export_type,
            inkspace_dir,
        );
    });

    can_exit_wating_screen();
}

// open the file manager on os the user is using
fn open_file_explorer(path: &str) {
    #[cfg(target_os = "windows")]
    {
        if let Err(err) = Command::new("explorer").arg(path).spawn() {
            println!("Failed to open File Explorer: {}", err);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if ({
            Err(err) = Command::new("open").arg(path).spawn();
        }) {
            println!("Failed to open Finder: {}", err);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if ({
            Err(err) = Command::new("xdg-open").arg(path).spawn();
        }) {
            println!("Failed to open file manager: {err}");
        }
    }
}

fn convert_file(input_file: &str, output_file: &str, export_type: &str, inkscape_path: &str) {
    // Construct the Inkscape command
    let mut cmd = Command::new(inkscape_path);
    let vector_formats: HashSet<&str> = ["svg", "pdf"]
        .iter()
        .cloned()
        .collect();

    cmd.arg(input_file) // Input file
        .arg("--export-filename") // Specify the output file
        .arg(output_file)
        .arg(format!("--export-type={}", export_type)); // Export type

    if vector_formats.contains(export_type) {
        cmd.arg("--export-dpi=600"); // Set DPI (useful for raster formats)
    }

    let status = cmd.status();

    match status {
        Ok(s) if s.success() => println!("Converted: {} -> {}", input_file, output_file),
        _ => eprintln!("Failed to convert: {} -> {}", input_file, output_file),
    }
}

fn selected_format(format_inde: &i32) -> String {
    // Map format to extension and image format
    let extension = match format_inde {
        0 => "svg",
        1 => "pdf",
        2 => "png",
        _ => "",
    };
    extension.to_string()
}
