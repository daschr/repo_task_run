use core::str;
use log::error;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, process::Command};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ExecutionContext {
    System,
    User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TaskType {
    OneShot,
    OnBoot,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    pub type_: TaskType,
    pub name: String,
    pub depends_on: Option<HashSet<String>>,
    pub context: ExecutionContext,
    pub user_filter: Option<HashSet<String>>,
    pub group_filter: Option<HashSet<String>>,
    pub reboot_required: bool,
    pub executable: PathBuf,
    pub hash: String,
}

impl Task {
    pub fn run(&self) -> bool {
        match Command::new("powershell.exe")
            .current_dir(self.executable.parent().unwrap())
            .arg("-WindowStyle")
            .arg("hidden")
            .arg(format!(
                "Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy Bypass; & \"{}\"",
                self.executable.display()
            ))
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    error!(
                        "Exit status of task \"{}\" (path: {}) is nonzero: {}",
                        self.name,
                        self.executable.display(),
                        output.status
                    );
                    error!(
                        "Stderr: {}",
                        str::from_utf8(&output.stderr).unwrap_or("<UTF8 Error>")
                    );
                    error!(
                        "Stdout: {}",
                        str::from_utf8(&output.stdout).unwrap_or("<UTF8 Error>")
                    );
                    false
                } else {
                    true
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to spawn process for task {} (execeutable: {}): {:?}",
                    self.name,
                    self.executable.display(),
                    e
                );
                false
            }
        }
    }
}

#[derive(Debug)]
pub struct Tasks(pub Vec<Task>);
