use crate::file_storage::FileStorageConfig;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::usize;
use std::{error::Error, path, process::Command};
use tokio::fs;

pub(crate) async fn create_mount(
    config: &FileStorageConfig,
) -> Result<Option<String>, Box<dyn Error>> {
    // first we will check using fs if the dir is availe or not if this is available we will just
    // return
    if path::Path::new(&config.root).exists() {
        let dir_metadata = fs::metadata(&config.root).await?;
        if !dir_metadata.is_dir() {
            return Err("Provided path already in use and not a directory".into());
        }
        // as dir already exist we can return
        return Ok(None);
    }
    fs::create_dir_all(&config.root).await?;
    if !config.create_mount {
        return Ok(None);
    }
    // #[cfg(target_os = "linux")]
    {
        use std::process::Command;

        let size = format!("size={}m", config.mount_size_in_mega_byte);
        let status = Command::new("mount")
            .args(&["-t", "tmpfs", "-o", &size])
            .arg(&config.root)
            .status()?;
        if !status.success() {
            return Err("Error while creating a mountpoint".into());
        }
        return Ok(Some(config.root.clone()));
    }
    #[cfg(target_os = "macos")]
    {
        use std::{process::Command, time};

        let blocks = config.mount_size_in_mega_byte * 2048; // 512-byte blocks
        println!("Creating this {}, blocks", blocks);
        let output = Command::new("hdiutil")
            .args(["attach", "-nomount", &format!("ram://{}", blocks)])
            .output()?;
        let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("device name is {}", device);
        let mount_point = format!("datanode"); // time::SystemTime::now());
        let mount_status = Command::new("diskutil")
            .args(["erasevolume", "HFS+", &mount_point, &device])
            .status()?;
        println!("Checking status {}", mount_status);
        if !mount_status.success() {
            return Err("Error while creating a mount point".into());
        }
        // remove the ./test directly but keep the parrents
        Command::new("rm").args(["-rf", &config.root]).status()?;
        if !Command::new("ln")
            .args(["-s", &format!("/Volumes/{}", &mount_point), &config.root])
            .status()?
            .success()
        {
            return Err("Error while linking the root to volume".into());
        }
        Ok(Some(device))
    }
}

// skiping the removal part
pub(crate) fn detach_device(device: &str) -> Result<(), Box<dyn Error>> {
    let status = if cfg!(target_os = "macos") {
        Command::new("hdiutil").args(["detach", device]).status()
    } else if cfg!(target_os = "linux") {
        Command::new("unmount").arg(device).status()
    } else {
        panic!("Unsupported OS for unmounting");
    };
    if !status?.success() {
        return Err(format!("Failed to unmount {}", device).into());
    }
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn available_storage(path: &str) -> Result<usize, Box<dyn Error + Send + Sync>> {
    use nix::sys::statvfs;
    use std::path::Path;

    let stats = statvfs::statvfs(Path::new(path))?;
    let available_bytes = stats.blocks_available() as usize * stats.fragment_size() as usize;
    println!("available storage {}", available_bytes);
    return Ok(available_bytes);
}
