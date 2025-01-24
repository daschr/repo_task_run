use crate::{
    common::{get_system_repository_path, get_user_repository_path, is_host_reachable, REPO_HOST},
    entra_groups::get_entra_groups_of_user,
    gix_repository::update_repo,
    task::{ExecutionContext, Task, TaskType, Tasks},
};
use log::{info, warn};
use sha256::TrySha256Digest;
use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum TaskFetchterError {
    CircularDependecy,
}

impl Display for TaskFetchterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskFetcherError")
    }
}

impl Error for TaskFetchterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        match self {
            TaskFetchterError::CircularDependecy => {
                "Tasks with circular dependencies cannot be ordered."
            }
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

pub struct TaskFetcher();

#[derive(Debug)]
pub struct StackEntry {
    path: PathBuf,
    tasktype: Option<TaskType>,
    context: Option<ExecutionContext>,
    depends_on: Option<HashSet<String>>,
    user_filter: Option<HashSet<String>>,
    group_filter: Option<HashSet<String>>,
    reboot_required: bool,
}

impl TaskFetcher {
    pub fn fetch_tasks(
        wanted_execution_context: ExecutionContext,
        upn: Option<String>,
    ) -> Result<(Tasks, bool), Box<dyn Error>> {
        let repo_path = match wanted_execution_context {
            ExecutionContext::System => get_system_repository_path()?,
            ExecutionContext::User => get_user_repository_path()?,
        };

        info!("Updating repo...");
        let has_changed = {
            while !is_host_reachable(REPO_HOST) {
                warn!(
                    "Host {} not reachable, trying to clone again in 10 seconds...",
                    REPO_HOST
                );
                std::thread::sleep(Duration::from_secs(10));
            }
            update_repo(&repo_path)?
        };

        info!("Building tasks from repo...");

        match Self::build_tasks_from_directory(&repo_path, wanted_execution_context, upn) {
            Some(tasks) => Ok((tasks, has_changed)),
            None => Err(Box::new(TaskFetchterError::CircularDependecy)),
        }
    }

    pub fn build_tasks_from_directory(
        dir: &Path,
        wanted_execution_context: ExecutionContext,
        upn: Option<String>,
    ) -> Option<Tasks> {
        let user_group_membership: Option<HashSet<String>> = match &upn {
            Some(u) => get_entra_groups_of_user(u).ok(),
            None => None,
        };

        info!(
            "user_group_membership of {:?}:{:?}",
            &upn, user_group_membership
        );

        let mut tasks: Vec<Task> = Vec::new();

        let mut stack: Vec<StackEntry> = Vec::new();
        stack.push(StackEntry {
            path: dir.to_path_buf(),
            tasktype: None,
            context: None,
            depends_on: None,
            user_filter: None,
            group_filter: None,
            reboot_required: false,
        });

        while !stack.is_empty() {
            let mut entry = stack.pop().unwrap();

            if entry.path.is_file() {
                match &entry.context {
                    None => {
                        info!("context is None, skipping {:?}", entry);
                        continue;
                    }
                    Some(c) if c != &wanted_execution_context => {
                        info!(
                            "Execution context is not {:?}, skipping {:?}",
                            wanted_execution_context, entry
                        );
                        continue;
                    }
                    _ => (),
                }

                if entry.tasktype.is_none() {
                    info!("tasktype is None, skipping {:?}", entry);
                    continue;
                }
                if let Some(e) = entry.path.as_path().extension() {
                    if e != "ps1" {
                        info!(
                            "Skipping {}, since it is not a .ps1",
                            entry.path.as_path().display()
                        );
                        continue;
                    }
                } else {
                    info!(
                        "Skipping {}, since it has no extension",
                        entry.path.as_path().display()
                    );
                    continue;
                }

                if matches!(entry.context.as_ref().unwrap(), ExecutionContext::User)
                    && entry.group_filter.is_some()
                {
                    if let Some(user_group_membership) = &user_group_membership {
                        if !entry
                            .group_filter
                            .as_ref()
                            .unwrap()
                            .iter()
                            .any(|req_group| user_group_membership.contains(req_group))
                        {
                            info!("Skipping {:?}, since the user is in none of the required groups ({:?})", &entry, user_group_membership);
                            continue;
                        }
                    } else {
                        info!("Skipping {:?}, since the user is in none of the required groups (no group membership)", &entry);
                        continue;
                    }
                }

                let digest = entry
                    .path
                    .as_path()
                    .digest()
                    .expect("Failed to calculate hash");

                let task_name = entry
                    .path
                    .as_path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .trim_end_matches(".ps1")
                    .to_string();

                tasks.push(Task {
                    type_: entry.tasktype.as_ref().unwrap().clone(),
                    name: task_name,
                    context: entry.context.as_ref().unwrap().clone(),
                    depends_on: entry.depends_on,
                    user_filter: entry.user_filter,
                    group_filter: entry.group_filter,
                    executable: entry.path,
                    reboot_required: entry.reboot_required,
                    hash: digest,
                })
            } else if entry.path.is_dir() {
                match fs::read_dir(entry.path.as_path()) {
                    Ok(dir) => {
                        if let Some(sn) = entry.path.as_path().file_name() {
                            let sn = sn.to_str().unwrap();
                            let mut spl = sn.splitn(2, "-");
                            match (spl.next(), spl.next()) {
                                (Some("group"), Some(name)) => {
                                    if entry.group_filter.is_none() {
                                        entry.group_filter = Some(HashSet::new());
                                    }
                                    entry
                                        .group_filter
                                        .as_mut()
                                        .unwrap()
                                        .insert(name.to_string());
                                }
                                (Some("user"), Some(name)) => {
                                    if entry.user_filter.is_none() {
                                        entry.user_filter = Some(HashSet::new());
                                    }
                                    entry.user_filter.as_mut().unwrap().insert(name.to_string());
                                }
                                (Some("depends"), Some(name)) => {
                                    if entry.depends_on.is_none() {
                                        entry.depends_on = Some(HashSet::new());
                                    }
                                    entry.depends_on.as_mut().unwrap().insert(name.to_string());
                                }
                                (Some("context"), Some("system")) => {
                                    entry.context = Some(ExecutionContext::System);
                                }
                                (Some("context"), Some("user")) => {
                                    entry.context = Some(ExecutionContext::User);
                                }
                                (Some("reboot"), Some("enabled")) => {
                                    entry.reboot_required = true;
                                }
                                (Some("reboot"), Some("disabled")) => {
                                    entry.reboot_required = false;
                                }
                                (Some("type"), Some("oneshot")) => {
                                    entry.tasktype = Some(TaskType::OneShot);
                                }
                                (Some("type"), Some("onboot")) => {
                                    entry.tasktype = Some(TaskType::OnBoot);
                                }
                                (_, _) => (),
                            }
                        }

                        for e in dir {
                            match e {
                                Ok(e) => {
                                    stack.push(StackEntry {
                                        path: e.path().to_path_buf(),
                                        tasktype: entry.tasktype.clone(),
                                        context: entry.context.clone(),
                                        depends_on: entry.depends_on.clone(),
                                        user_filter: entry.user_filter.clone(),
                                        group_filter: entry.group_filter.clone(),
                                        reboot_required: entry.reboot_required,
                                    });
                                }
                                Err(err) => {
                                    info!("Error reading directory entry: {:?}", err);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        info!("Error listing directoriy entries: {:?}", e);
                    }
                }
            }
        }

        if let Some(ordered_tasks) = Self::order_tasks_by_dependency(&tasks) {
            Some(Tasks(ordered_tasks))
        } else {
            None
        }
    }

    fn order_tasks_by_dependency(orig_tasks: &[Task]) -> Option<Vec<Task>> {
        let mut ordered_tasks: HashSet<String> = HashSet::new();
        let mut unordered_tasks: VecDeque<usize> = VecDeque::new();

        for i in 0..orig_tasks.len() {
            unordered_tasks.push_back(i);
        }

        let mut iteration_count = 0;
        let mut tasks = Vec::new();

        'outer: while !unordered_tasks.is_empty() {
            info!("Ordered_tasks: {:?}", ordered_tasks);
            info!("Unordered_tasks: {:?}", unordered_tasks);

            let c_task = unordered_tasks.pop_front().unwrap();

            info!("Got task: {:?}", orig_tasks[c_task]);
            if let Some(dependencies) = orig_tasks[c_task].depends_on.as_ref() {
                for dep in dependencies {
                    if !ordered_tasks.contains(dep) {
                        if iteration_count >= unordered_tasks.len() {
                            info!("Circular dependency detected, giving up!");
                            return None;
                        }

                        iteration_count += 1;
                        unordered_tasks.push_back(c_task);
                        continue 'outer;
                    }
                }
            }

            iteration_count = 0;
            ordered_tasks.insert(orig_tasks[c_task].name.clone());
            tasks.push(orig_tasks[c_task].clone());
        }

        Some(tasks)
    }
}
