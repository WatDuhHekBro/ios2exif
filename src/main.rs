use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    process::Command,
    str,
};

// "CreationDate" is actually QuickTime metadata, not EXIF metadata. (https://superuser.com/a/1285932)
// "-s3" is to just get the value without the header. (https://photo.stackexchange.com/a/56678)
// Example: "exiftool -DateTimeOriginal -s3 2023-05-25_19-47-30.heic" gives value "2023:05:25 19:47:30"
// Example: "exiftool -CreationDate -s3 2023-05-14_21-34-06.mov" gives value "2023:05:14 21:34:06-05:00"
// Example: "exiftool -CreateDate -s3 2023-05-14_21-34-06.mp4" gives value "2023:05:14 21:34:06" (timezone-unaware, manual checking required)

struct FileInfo {
    path: String,
    new_name: String,
}

// Maybe implement command line arguments that target specific files.
fn main() {
    println!(
        "-=[ {} - v{} ]=-",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    // Get file iterator of current directory
    let Ok(current_directory) = env::current_dir() else {
        return eprintln!(
            "The current working directory either doesn't exist or isn't accessible."
        );
    };
    let Ok(files) = fs::read_dir(current_directory) else {
        return eprintln!("The current working directory isn't a valid directory.");
    };
    let mut needs_confirmation = false;
    let mut must_exit = false;
    let mut map = BTreeMap::<String, FileInfo>::new(); // Map<timestamp, path>, used to test for duplicate timestamps.

    for file in files {
        // Ignore the file if it can't be read
        let Ok(file) = file else {
            eprintln!("Warning: A file can't be read.");
            needs_confirmation = true;
            continue;
        };
        let path = file.path();
        let path_str = path.to_string_lossy().to_string();
        // Get lowercase extension (if any)
        let extension = {
            let extension = path.extension();

            match extension {
                Some(extension) => Some(extension.to_string_lossy().to_lowercase()),
                None => None,
            }
        };

        // Ignore directories
        if path.is_dir() {
            continue;
        }

        // Determine what to do based on the file extension
        let try_exif_first = match extension {
            Some(ref extension) => match extension.as_str() {
                // Photos
                "jpg" => true,
                "jpeg" => true,
                "png" => true,
                "heic" => true,
                // Videos
                "mov" => false,
                "mp4" => false,
                _ => {
                    println!("Warning: Unsupported extension \".{extension}\", ignoring...");
                    continue;
                }
            },
            _ => true,
        };

        let result = get_timestamp_and_rename_pair(&path, &path_str, extension, try_exif_first);
        let Some(result) = result else {
            needs_confirmation = true;
            continue;
        };
        let (timestamp, new_name) = result;

        // Error if two files have the same timestamp, as that will definitely cause problems.
        // Continue the loop to show all occurrences.
        if map.contains_key(&timestamp) {
            eprintln!(
                    "Error: Attempted to add \"{path_str}\"\n\t...but the timestamp ({timestamp}) already exists in file: \"{}\"",
                    map[&timestamp].path
                );
            must_exit = true;
        } else {
            map.insert(
                timestamp,
                FileInfo {
                    path: path_str,
                    new_name,
                },
            );
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
    for (timestamp, info) in map {
        let result = fs::rename(&info.path, &info.new_name);

        if let Err(error) = result {
            eprintln!("Error: Renaming failed for \"{}\" - {error}", info.path);
        } else {
            println!(
                "Renaming success for \"{}\" to timestamp \"{timestamp}\".",
                info.path
            );
        }
    }
}

// Try exif first, otherwise use "exiftool" to read miscellaneous metadata (QuickTime, etc.)
// Returns a pair of (timestamp, new_name) if successful
fn get_timestamp_and_rename_pair(
    path: &PathBuf,
    path_str: &String,
    extension: Option<String>,
    try_exif_first: bool,
) -> Option<(String, String)> {
    if try_exif_first {
        let timestamp = get_timestamp_from_exif(path, path_str);

        match timestamp {
            Ok(timestamp) => {
                if let Some(extension) = extension {
                    return Some((timestamp.clone(), format!("{timestamp}.{extension}")));
                } else {
                    return Some((timestamp.clone(), timestamp));
                }
            }
            Err(error_message) => {
                eprintln!("{}", error_message);
            }
        }
    }

    // Try CreationDate
    let timestamp = get_timestamp_from_exiftool_creationdate(path_str);

    if let Some(timestamp) = timestamp {
        if let Some(extension) = extension {
            return Some((timestamp.clone(), format!("{timestamp}.{extension}")));
        } else {
            return Some((timestamp.clone(), timestamp));
        }
    };

    // Try CreateDate with warning in filename
    let timestamp = get_timestamp_from_exiftool_createdate(path_str);

    if let Some(timestamp) = timestamp {
        if let Some(extension) = extension {
            return Some((timestamp.clone(), format!("{timestamp} (utc).{extension}",)));
        } else {
            return Some((timestamp.clone(), format!("{timestamp} (utc)")));
        }
    };

    // Nothing found otherwise
    None
}

// Returns a string of the formatted timestamp if successful
// Ignore the entry if unsuccessful
fn get_timestamp_from_exif(path: &PathBuf, path_str: &String) -> Result<String, String> {
    let Ok(file) = File::open(path) else {
        return Err(format!(
            "Warning: Failed to open the file: \"{}\"",
            path_str
        ));
    };
    let mut bufreader = BufReader::new(&file);
    let exifreader = exif::Reader::new();

    // Ignore the file if invalid exif
    let Ok(exif) = exifreader.read_from_container(&mut bufreader) else {
        return Err(format!(
            "Warning: The file doesn't contain valid EXIF: \"{}\"",
            path_str
        ));
    };

    // Extract the EXIF field DateTimeOriginal, use it as the new filename.
    // Do not use DateTime, as that's supposed to update if the image is modified (https://gitlab.gnome.org/GNOME/gimp/-/issues/8160).
    // Ignore if "ifd_num" isn't "primary", as that indicates that it's a thumbnail image, not a main image.
    let Some(datetime) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) else {
        return Err(format!("Warning: EXIF metadata is present but does not include DateTimeOriginal for file \"{path_str}\". Not renaming..."));
    };

    // Convert timestamp of format "YYYY-MM-DD HH:MM:SS" to "YYYY-MM-DD_HH-MM-SS"
    let mut timestamp = datetime.display_value().to_string();
    // You don't actually need to use regex for this, just replace spaces and colons.
    timestamp = timestamp.replace(" ", "_");
    timestamp = timestamp.replace(":", "-");

    Ok(timestamp)
}

// If it fails for whatever reason, just ignore the entry
fn get_timestamp_from_exiftool_creationdate(path_str: &String) -> Option<String> {
    // Format: "YYYY:MM:DD HH:MM:SS-ZZ:00"
    let output = Command::new("exiftool")
        .arg("-CreationDate")
        .arg("-s3")
        .arg(path_str)
        .output();

    let Ok(output) = output else {
        eprintln!(
            "[exiftool] Warning: Failed to execute \"exiftool\" process on path \"{path_str}\"!"
        );
        return None;
    };

    let length = output.stdout.len();

    // Output is empty if metadata attribute doesn't exist
    if length <= 0 {
        eprintln!("[exiftool] Warning: No output for tag \"CreationDate\" on path \"{path_str}\"!");
        return None;
    }

    // -1 for ending newline
    // -6 for timezone
    let slice = &output.stdout[0..length - 7];
    let timestamp = str::from_utf8(slice);

    let Ok(timestamp) = timestamp else {
        eprintln!(
            "[exiftool] Warning: CreationDate \"{:?}\" should be a valid UTF-8 string on path \"{path_str}\"!",
            slice
        );
        return None;
    };

    let mut timestamp = timestamp.to_string();
    // Convert timestamp of format "YYYY:MM:DD HH:MM:SS" to "YYYY-MM-DD_HH-MM-SS"
    timestamp = timestamp.replace(" ", "_");
    timestamp = timestamp.replace(":", "-");

    Some(timestamp)
}

// If it fails for whatever reason, just ignore the entry
fn get_timestamp_from_exiftool_createdate(path_str: &String) -> Option<String> {
    // Format: "YYYY:MM:DD HH:MM:SS"
    let output = Command::new("exiftool")
        .arg("-CreateDate")
        .arg("-s3")
        .arg(path_str)
        .output();

    let Ok(output) = output else {
        eprintln!(
            "[exiftool] Warning: Failed to execute \"exiftool\" process on path \"{path_str}\"!"
        );
        return None;
    };

    let length = output.stdout.len();

    // Output is empty if metadata attribute doesn't exist
    if length <= 0 {
        eprintln!("[exiftool] Warning: No output for tag \"CreateDate\" on path \"{path_str}\"!");
        return None;
    }

    // -1 for ending newline
    let slice = &output.stdout[0..length - 1];
    let timestamp = str::from_utf8(slice);

    let Ok(timestamp) = timestamp else {
        eprintln!(
            "[exiftool] Warning: CreateDate \"{:?}\" should be a valid UTF-8 string on path \"{path_str}\"!",
            slice
        );
        return None;
    };

    let mut timestamp = timestamp.to_string();
    // Convert timestamp of format "YYYY:MM:DD HH:MM:SS" to "YYYY-MM-DD_HH-MM-SS"
    timestamp = timestamp.replace(" ", "_");
    timestamp = timestamp.replace(":", "-");

    Some(timestamp)
}
