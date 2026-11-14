use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub kernel_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub cacert: PathBuf,
    pub git_backup_file: PathBuf,
}

pub fn paths_from_base(base: PathBuf) -> AppPaths {
    let data_dir = base.join("FastGithubGUI");
    let kernel_dir = data_dir.join("kernel");
    let logs_dir = data_dir.join("logs");
    let cacert = kernel_dir.join("cacert").join("fastgithub.cer");
    let git_backup_file = data_dir.join("git-backup.json");
    AppPaths { data_dir, kernel_dir, logs_dir, cacert, git_backup_file }
}

pub fn base_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        Path::new(&home).join("Library").join("Application Support")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(std::env::var("APPDATA").expect("APPDATA not set"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        Path::new(&home).join(".local").join("share")
    }
}

pub fn app_paths() -> AppPaths {
    paths_from_base(base_dir())
}

pub fn session_file() -> PathBuf {
    app_paths().data_dir.join("state.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout() {
        let p = paths_from_base(PathBuf::from("/tmp/base"));
        assert_eq!(p.kernel_dir, PathBuf::from("/tmp/base/FastGithubGUI/kernel"));
        assert_eq!(p.cacert, PathBuf::from("/tmp/base/FastGithubGUI/kernel/cacert/fastgithub.cer"));
        assert_eq!(p.logs_dir, PathBuf::from("/tmp/base/FastGithubGUI/logs"));
        assert_eq!(p.git_backup_file, PathBuf::from("/tmp/base/FastGithubGUI/git-backup.json"));
    }
}