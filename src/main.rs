use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::BufReader,
    path::Path,
};

// Maybe implement command line arguments that target specific files.
fn main() {
    println!(
        "-=[ {} - v{} ]=-",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let Ok(current_directory) = env::current_dir() else {
        return eprintln!("The current working directory either doesn't exist or isn't accessible.");
    };
    let Ok(files) = fs::read_dir(current_directory) else {
        return eprintln!("The current working directory isn't a valid directory.");
    };
    let mut needs_confirmation = false;
    let mut must_exit = false;
    let mut map = BTreeMap::<String, String>::new(); // Map<timestamp, path>, used to test for duplicate timestamps.

    for file in files {
        // Ignore the file if it can't be read
        let Ok(file) = file else {
            eprintln!("Warning: A file can't be read.");
            needs_confirmation = true;
            continue;
        };
        let path = file.path();
        let path_str = path.to_string_lossy().to_string();

        // Ignore directories
        if !path.is_dir() {
            let Ok(file) = File::open(&path) else {
                eprintln!("Warning: Failed to open the file: \"{}\"", path_str);
                needs_confirmation = true;
                continue;
            };
            let mut bufreader = BufReader::new(&file);
            let exifreader = exif::Reader::new();

            // Ignore the file if invalid exif
            let Ok(exif) = exifreader.read_from_container(&mut bufreader) else {
                eprintln!("Warning: The file doesn't contain valid EXIF: \"{}\"", path_str);
                needs_confirmation = true;
                continue;
            };

            // Extract the EXIF field DateTimeOriginal, use it as the new filename.
            // Do not use DateTime, as that's supposed to update if the image is modified (https://gitlab.gnome.org/GNOME/gimp/-/issues/8160).
            // Ignore if "ifd_num" isn't "primary", as that indicates that it's a thumbnail image, not a main image.
            let Some(datetime) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) else {
                eprintln!("Warning: EXIF metadata is present but does not include DateTimeOriginal for file \"{path_str}\". Not renaming...");
                needs_confirmation = true;
                continue;
            };

            // You don't actually need to use regex for this, just replace spaces and colons.
            let mut timestamp = datetime.display_value().to_string(); // Format: "YYYY-MM-DD HH:MM:SS"
            timestamp = timestamp.replace(" ", "_");
            timestamp = timestamp.replace(":", "-");

            // Error if two files have the same timestamp, as that will definitely cause problems.
            // Continue the loop to show all occurrences.
            if map.contains_key(&timestamp) {
                eprintln!(
                    "Error: Attempted to add \"{path_str}\"\n\t...but the timestamp ({timestamp}) already exists in file: \"{}\"",
                    map[&timestamp]
                );
                must_exit = true;
            } else {
                map.insert(timestamp, path_str);
            }
        }
    }

    if must_exit {
        eprintln!("Error: Found conflicting timestamps, exiting...");
        return;
    }

    if needs_confirmation {
        use text_io::read;

        // Ask Y/y/N/n, exit otherwise.
        print!("Are all the warnings are okay with you? [y/n] ");
        let response: String = read!();

        match response.as_str() {
            "Y" => {}
            "y" => {}
            "N" => {
                return println!("Exiting...");
            }
            "n" => {
                return println!("Exiting...");
            }
            _ => {
                return eprintln!("Invalid response, exiting...");
            }
        }
    }

    // Once confirmed or no warnings, then proceed with the renaming.
    for (timestamp, path) in map {
        let extension = Path::new(&path).extension();

        let result = {
            if let Some(extension) = extension {
                fs::rename(
                    &path,
                    format!("{timestamp}.{}", extension.to_string_lossy().to_lowercase()),
                )
            } else {
                fs::rename(&path, &timestamp)
            }
        };

        if let Err(error) = result {
            eprintln!("Error: Renaming failed for \"{path}\" - {error}");
        } else {
            println!("Renaming success for \"{path}\" to timestamp \"{timestamp}\".");
        }
    }
}
