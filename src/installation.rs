use log::{error, info};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use windows::Win32::Storage::FileSystem::MOVEFILE_DELAY_UNTIL_REBOOT;
use windows::{core::PCWSTR, Win32::Storage::FileSystem::MoveFileExW};
use windows_registry::CURRENT_USER;

use crate::common::*;

pub trait AutostartConfiguration {
    fn uninstall() -> Result<(), Box<dyn Error>>;
    fn install() -> Result<(), Box<dyn Error>>;
}

pub struct PerUserAutostart();

impl AutostartConfiguration for PerUserAutostart {
    fn uninstall() -> Result<(), Box<dyn Error>> {
        let mut winadm_path = get_user_install_path()?;

        let own_path = PathBuf::from(std::env::args().next().unwrap());

        winadm_path.push(own_path.as_path().file_name().unwrap());

        if let Err(e) = fs::remove_file(&winadm_path) {
            error!("Failed to remove {}: {:?}", winadm_path.display(), e);
        }

        let key = CURRENT_USER.create("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;

        key.remove_value(RUN_REGKEY_NAME).ok();

        Ok(())
    }

    fn install() -> Result<(), Box<dyn Error>> {
        let mut winadm_path = get_user_install_path()?;

        if !winadm_path.exists() {
            info!(
                "{} does not exist, creating directory...",
                winadm_path.display()
            );
            fs::create_dir_all(&winadm_path)?;
        }

        let own_path = PathBuf::from(std::env::args().next().unwrap());

        winadm_path.push(own_path.as_path().file_name().unwrap());

        if own_path != winadm_path {
            info!(
                "Copying {} to {}",
                own_path.display(),
                winadm_path.display()
            );

            fs::copy(&own_path, &winadm_path)?;
        }

        info!("Opening key...");
        let key = CURRENT_USER.create("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;

        info!("Setting {}", RUN_REGKEY_NAME);
        key.set_string(
            RUN_REGKEY_NAME,
            &format!("\"{}\"", winadm_path.to_str().unwrap()),
        )?;

        info!("Regkey set");
        Ok(())
    }
}

pub struct SystemAutostart();

impl AutostartConfiguration for SystemAutostart {
    fn uninstall() -> Result<(), Box<dyn Error>> {
        let mut install_path = get_system_install_path()?;

        let own_path = PathBuf::from(std::env::args().next().unwrap());

        install_path.push(own_path.as_path().file_name().unwrap());

        if install_path.exists() {
            info!("Removing {} ...", install_path.display());
            let install_path_s: Vec<u16> = install_path.to_string_lossy().encode_utf16().collect();

            unsafe {
                MoveFileExW(
                    PCWSTR::from_raw(install_path_s.as_ptr()),
                    PCWSTR::null(),
                    MOVEFILE_DELAY_UNTIL_REBOOT,
                )?;
            }
        }

        Command::new("schtasks.exe")
            .arg("/delete")
            .arg("/TN")
            .arg(APP_NAME)
            .arg("/F")
            .output()
            .ok();

        Ok(())
    }

    fn install() -> Result<(), Box<dyn Error>> {
        info!("Installing to SYSTEM");
        let mut install_path = get_system_install_path()?;

        if !install_path.exists() {
            info!(
                "{} does not exist, creating directory...",
                install_path.display()
            );
            fs::create_dir_all(&install_path)?;
        }

        let own_path = PathBuf::from(std::env::args().next().unwrap());

        install_path.push(own_path.as_path().file_name().unwrap());

        if own_path != install_path {
            info!(
                "Copying {} to {}",
                own_path.display(),
                install_path.display()
            );

            fs::copy(&own_path, &install_path)?;
        }

        info!("Removing scheduled task, if it exists...");

        Command::new("schtasks.exe")
            .arg("/delete")
            .arg("/TN")
            .arg(APP_NAME)
            .arg("/F")
            .output()
            .ok();

        info!("Adding scheduled task...");

        Command::new("schtasks.exe")
            .arg("/Create")
            .arg("/ru")
            .arg("system")
            .arg("/TR")
            .arg(install_path)
            .arg("/TN")
            .arg(APP_NAME)
            .arg("/SC")
            .arg("ONSTART")
            .arg("/F")
            .output()
            .ok();

        Ok(())
    }
}
