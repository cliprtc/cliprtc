use std::{
    fs::{create_dir_all, File},
    io::{self, BufReader},
    path::{Component, Path, PathBuf},
};

use tar::{Archive, Builder};
use tauri_plugin_log::log;

pub fn decompress_path(pack_path: &Path, target_dir: &Path) -> io::Result<()> {
    let file = File::open(pack_path)?;
    let reader = BufReader::new(file);
    let mut archive = Archive::new(reader);

    archive.unpack(target_dir)?;

    Ok(())
}

pub fn compress_paths(paths: &[impl AsRef<Path>], pack_path: &Path) -> io::Result<()> {
    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No paths to compress",
        ));
    }

    if let Some(base_path) = find_common_base_path(paths) {
        if let Some(parent) = pack_path.parent() {
            create_dir_all(parent)?;
        }

        let pack_file = File::create(pack_path)?;
        let mut pack = Builder::new(pack_file);

        for path in paths {
            let path = path.as_ref();
            if !path.exists() {
                log::warn!("Path does not exist, skipping: {}", path.display());
                continue;
            }

            let relative_path = path.strip_prefix(&base_path).unwrap_or(path);

            if path.is_file() {
                log::debug!("Adding single file: {}", relative_path.display());

                let mut f = File::open(path)?;
                pack.append_file(relative_path, &mut f)?;
            } else if path.is_dir() {
                log::debug!("Adding directory: {}", relative_path.display());
                pack.append_dir_all(relative_path, path)?;
            } else {
                log::warn!("Skipping invalid path type: {}", path.display());
            }
        }

        log::info!("Created pack at {}", pack_path.display());
        // pack.into_inner()?;
        pack.finish()?;
    }

    Ok(())
}

fn find_common_base_path<P: AsRef<Path>>(paths: &[P]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    // Single path: return its parent directory
    if paths.len() == 1 {
        return paths[0].as_ref().parent().map(|p| p.to_path_buf());
    }

    // Split all paths into components
    let split_paths: Vec<Vec<Component>> = paths
        .iter()
        .map(|p| p.as_ref().components().collect())
        .collect();

    // Find the longest common prefix of components
    let mut common: Vec<Component> = Vec::new();
    for (i, c) in split_paths[0].iter().enumerate() {
        if split_paths.iter().all(|p| p.get(i) == Some(c)) {
            common.push(*c);
        } else {
            break;
        }
    }

    if common.is_empty() {
        return None;
    }

    // Reconstruct the common path from components
    let mut base = PathBuf::new();
    for c in common {
        base.push(c.as_os_str());
    }
    Some(base)
}
