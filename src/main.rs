// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::{open, DynamicImage, ImageBuffer, ImageFormat};
use rayon::prelude::*;
use rfd::FileDialog;
use slint::{LogicalSize, Model, ModelRc, SharedString, ToSharedString, VecModel, WindowSize};
use std::error::Error;
use std::process::Command;
use std::sync::{Arc, Mutex};

slint::include_modules!();
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
    select_file_format(&app);
}

/// Button Selection Image
fn button_selection_image(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    if let Some(app_upgrade) = app_weak.upgrade() {
        app_upgrade
            .clone_strong()
            .on_ButtonSelectImageClicked(move || {
                if let Some(files) = FileDialog::new()
                    .add_filter(
                        "image",
                        // "Png","WebP","Bmp","Jpeg"
                        &["png", "bmp", "webp", "jpeg"],
                    )
                    .set_directory("/")
                    .pick_files()
                {
                    let files_vector: Vec<SharedString> = files
                        .par_iter()
                        .map(|file| file.as_path().to_string_lossy().to_shared_string())
                        .collect();

                    // Wrap the VecModel in ModelRc for shared ownership
                    let converted_files_path = ModelRc::new(VecModel::from(files_vector));
                    app_upgrade.set_image_path(converted_files_path);
                }
            });
    }
}

/// Button Selection Image Output Path
fn button_selection_image_output(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    if let Some(app_upgrade) = app_weak.upgrade() {
        app_upgrade
            .clone_strong()
            .on_ButtonSelectImageOutputClicked(move || {
                if let Some(file) = FileDialog::new().set_directory("/").pick_folder() {
                    println!("{}", file.display());
                    app_upgrade
                        .set_image_output_path(file.as_path().to_string_lossy().to_shared_string());
                }
            });
    }
}

/// Button Selection Image
fn button_apply_changes(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    app.on_ButtonApplyChangesClicked(move |paths, output_dir, name, format| {
        if let Some(_) = app_weak.upgrade() {
            let paths_vec: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
            process_images(
                paths_vec,
                &output_dir.to_string(),
                &name.to_string(),
                &format,
            );
        }
    });
}

// Process Images
fn process_images(
    paths: Vec<String>,
    output_path: &String,
    file_name: &String,
    format: &SupportedImageFormats,
) {
    let counter: Arc<Mutex<i16>> = Arc::new(Mutex::new(0)); // Create a Mutex-wrapped counter
    paths.par_iter().for_each(|path| {
        if let Ok(count) = counter.lock() {
            iter_paths(path, output_path, file_name, &format, *count);
        }

        if let Ok(mut counter_lock) = counter.lock() {
            // Lock the Mutex and increment the counter
            *counter_lock += 1;
        }
    });

    // the file manager when all images has finished converting
    open_file_explorer(output_path);
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
       if Err(err) =  Command::new("open").arg(path).spawn() {
            println!("Failed to open Finder: {}", err);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if Err(err) = Command::new("xdg-open").arg(path).spawn() {
            println!("Failed to open file manager: {err}");
        }
    }
}

// Iter Paths For Images
fn iter_paths(
    path: &String,
    output_dir: &String,
    name: &String,
    format: &SupportedImageFormats,
    counter: i16,
) {
    if let Ok(_image) = open(path.as_str()) {
        // Map format to extension and image format
        let (extension, image_format) = match format {
            slint_generatedAppWindow::SupportedImageFormats::Png => ("png", ImageFormat::Png),
            slint_generatedAppWindow::SupportedImageFormats::Jpeg => ("jpeg", ImageFormat::Jpeg),
            slint_generatedAppWindow::SupportedImageFormats::WebP => ("webp", ImageFormat::WebP),
            slint_generatedAppWindow::SupportedImageFormats::Bmp => ("bmp", ImageFormat::Bmp),
        };

        // Create the output file path
        let output_path = format!("{}/{}{}.{}", output_dir, name, counter, extension);

        // Save image in the selected format
        change_image_format(path, &image_format, &output_path);
    } else {
        eprintln!("Failed to open image: {}", path);
    }
}

/// Select File Format
fn select_file_format(app: &Arc<AppWindow>) {
    let app_weak = app.as_weak(); // Create a Weak reference to app
    if let Some(app_upgrade) = app_weak.upgrade() {
        app_upgrade
            .clone_strong()
            .on_SelectFileFormat(move |index| {
                if let Some(format) = match index {
                    0 => Some(slint_generatedAppWindow::SupportedImageFormats::Png),
                    1 => Some(slint_generatedAppWindow::SupportedImageFormats::WebP),
                    2 => Some(slint_generatedAppWindow::SupportedImageFormats::Bmp),
                    3 => Some(slint_generatedAppWindow::SupportedImageFormats::Jpeg),
                    _ => None, // Handle invalid indices explicitly
                } {
                    app_upgrade.set_format_selected(format);
                } else {
                    eprintln!("Invalid format index: {}", index);
                }
            });
    }
}

// Change Image Format
fn change_image_format(path: &String, format: &ImageFormat, output_path: &String) {
    match open(path) {
        Ok(image) => {
            convert_image_color(&format, image, output_path.as_str());
        }
        Err(err) => {
            eprintln!("Failed to open image: {}", err);
        }
    }
}

fn convert_image_color(format: &ImageFormat, image: DynamicImage, output_path: &str) {
    // "Png","WebP","Bmp","Jpeg"
    match format {
        ImageFormat::Jpeg => {
            let img = image.to_rgb8();
            convert_image_format_to_rbg8(&img, output_path, format);
        }
        _ => {
            convert_image_format(&image, output_path, format);
        }
    }
}

// Convert Image Format To RB8bit Images
fn convert_image_format_to_rbg8(
    img: &ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    output_path: &str,
    format: &ImageFormat,
) {
    match img.save_with_format(&output_path, *format) {
        Ok(_new_img) => {}
        Err(err) => {
            eprintln!("Filed to convert the image: {}", err);
        }
    }
}

// Convert Image Format
fn convert_image_format(img: &DynamicImage, output_path: &str, format: &ImageFormat) {
    match img.save_with_format(&output_path, *format) {
        Ok(_new_img) => {}
        Err(err) => {
            eprintln!("Filed to convert the image: {}", err);
        }
    }
}
