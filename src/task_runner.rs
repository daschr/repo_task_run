use crate::{
    common::{get_appdata_local, get_programdata, get_upn, APP_NAME},
    task::{ExecutionContext, Task, TaskType},
    task_fetcher::TaskFetcher,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    process::Command,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TaskRunner {
    done_oneshot_tasks: HashMap<String, String>,
    task_list: Vec<Task>,
    next_task: usize,
    execution_context: ExecutionContext,
}

use log::info;

impl TaskRunner {
    pub fn new(execution_context: ExecutionContext) -> Result<Self, Box<dyn Error>> {
        let upn = get_upn();
        let (fetched_tasks, tasks_changed) =
            TaskFetcher::fetch_tasks(execution_context.clone(), upn)?;

        match Self::restore_from_disk(&execution_context) {
            Some(mut restored_state) => {
                if tasks_changed {
                    restored_state.next_task = 0;
                    restored_state.task_list = fetched_tasks.0;

                    let mut hs = HashSet::new();
                    for t in &restored_state.task_list {
                        hs.insert(t.name.clone());
                    }

                    let restored_tasknames: Vec<String> =
                        restored_state.done_oneshot_tasks.keys().cloned().collect();

                    for task in restored_tasknames {
                        if !hs.contains(&task) {
                            info!("Removing task \"{}\" from restored done_oneshot_tasks since it does not exist anymore.", task);
                            restored_state.done_oneshot_tasks.remove(&task);
                        }
                    }
                }
                return Ok(restored_state);
            }
            None => {
                info!("Could not restore state");
                Ok(TaskRunner {
                    done_oneshot_tasks: HashMap::new(),
                    task_list: fetched_tasks.0,
                    next_task: 0,
                    execution_context,
                })
            }
        }
    }

    fn restore_from_disk(execution_context: &ExecutionContext) -> Option<Self> {
        match execution_context {
            ExecutionContext::System => {
                let mut path = get_programdata().unwrap();
                path.push(APP_NAME);
                path.push("state.bin");

                if !path.exists() || !path.is_file() {
                    return None;
                }

                if let Ok(buf) = fs::read(path) {
                    if let Ok(state) = bincode::deserialize(&buf) {
                        return Some(state);
                    }
                }

                None
            }
            ExecutionContext::User => {
                let mut path = get_appdata_local().unwrap();
                path.push(APP_NAME);
                path.push("state.bin");

                if !path.exists() || !path.is_file() {
                    return None;
                }

                if let Ok(buf) = fs::read(path) {
                    if let Ok(state) = bincode::deserialize(&buf) {
                        return Some(state);
                    }
                }

                None
            }
        }
    }

    fn store_to_disk(&self) {
        match self.execution_context {
            ExecutionContext::System => {
                let mut path = get_programdata().unwrap();
                path.push(APP_NAME);

                if !path.exists() {
                    fs::create_dir_all(&path).unwrap();
                }

                path.push("state.bin");

                let buf = bincode::serialize(self).unwrap();
                fs::write(path, &buf).unwrap();
            }
            ExecutionContext::User => {
                let mut path = get_appdata_local().unwrap();
                path.push(APP_NAME);

                if !path.exists() {
                    fs::create_dir_all(&path).unwrap();
                }

                path.push("state.bin");

                let buf = bincode::serialize(self).unwrap();
                fs::write(path, &buf).unwrap();
            }
        }
    }

    fn remove_disk_state(execution_context: &ExecutionContext) {
        match execution_context {
            ExecutionContext::System => {
                let mut path = get_programdata().unwrap();
                path.push(APP_NAME);
                path.push("state.bin");

                if path.exists() {
                    fs::remove_file(path).unwrap();
                }
            }
            ExecutionContext::User => {
                let mut path = get_appdata_local().unwrap();
                path.push(APP_NAME);
                path.push("state.bin");

                if path.exists() {
                    fs::remove_file(path).unwrap();
                }
            }
        }
    }

    pub fn run(&mut self) {
        while self.next_task < self.task_list.len() {
            let task = &self.task_list[self.next_task];

            if matches!(task.type_, TaskType::OneShot) {
                if let Some(old_hash) = self.done_oneshot_tasks.get(&task.name) {
                    if old_hash == &task.hash {
                        info!(
                            "Skipping execution of OneShot task: {}, since it has not changed",
                            task.name
                        );
                        self.next_task += 1;
                        continue;
                    }
                }
            }

            info!("Running task \"{}\"", task.name);

            if !task.run() {
                info!("TASK EXECUTION FAILED, GIVING UP!");
                Self::remove_disk_state(&self.execution_context);
                break;
            }

            if matches!(task.type_, TaskType::OneShot) {
                self.done_oneshot_tasks
                    .insert(task.name.clone(), task.hash.clone());
            }

            self.next_task += 1;

            if task.reboot_required {
                self.store_to_disk();

                Self::do_reboot();
            }
        }

        self.store_to_disk();

        if self.next_task >= self.task_list.len() {
            info!("ALL TASKS EXECUTED SUCCESSFULLY!");
        }
    }

    fn do_reboot() {
        Command::new("powershell.exe")
            .arg("Restart-Computer -Force")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }
}
