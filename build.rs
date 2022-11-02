use std::collections::hash_map::DefaultHasher;
use std::fs::{self, create_dir, rename};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{env, panic};

use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

use crate::config_const::{CONFIG_FOLDER, CONFIG_LOCAL};

pub mod config_const {
    pub const CONFIG_FOLDER: &str = "config";
    pub const CONFIG_LOCAL: &str = "local";
}

fn main() {
    let config_folder = Path::new(CONFIG_FOLDER);
    let target_folder = env::var_os("OUT_DIR").map(PathBuf::from).unwrap().join("../../..");
    let local_source = config_folder.join(CONFIG_LOCAL).with_extension("json");
    let local_backup_folder = config_folder.with_extension("backup");
    let local_backup = local_backup_folder.join(CONFIG_LOCAL);

    let t = target_folder.canonicalize();

    eprintln!("Target  : {target_folder:?} : {t:?}");

    // local setting
    if local_source.exists() {
        if local_backup_folder.exists() {
            if !local_backup_folder.is_dir() {
                panic!("Exists and is not a Folder!: {local_backup_folder:?}")
            }
        } else {
            create_dir(local_backup_folder).unwrap();
        }

        let mut hasher = DefaultHasher::new();

        fs::read_to_string(&local_source).unwrap().hash(&mut hasher);

        rename(
            local_source,
            local_backup.with_extension(format!("{}-{}", "toml", hasher.finish())),
        )
        .unwrap();
    }

    // Re-runs script if any files in config are changed
    // println!("cargo:rerun-if-changed=config/*");
    // copy_to_target(config_folder, &target_folder).expect("Could not copy");
}

pub fn copy_to_target(path: &Path, target: &Path) -> Result<u64, fs_extra::error::Error> {
    let mut options = CopyOptions::new();
    let mut paths = Vec::new();

    // Overwrite existing files with same name
    options.overwrite = true;

    paths.push(path);
    match copy_items(&paths, target, &options) {
        Ok(r) => Ok(r),
        Err(e) => {
            let f = path.canonicalize();
            let t = target.canonicalize();
            eprintln!("Tried to copy from: {f:?}");
            eprintln!("Tried to copy to  : {t:?}");
            Err(e)
        }
    }
}
