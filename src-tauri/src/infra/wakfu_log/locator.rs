use std::path::PathBuf;

/// Failure modes when resolving the Wakfu client's log file path.
///
/// Each variant is only ever constructed on the platform it targets, hence
/// the matching `#[cfg]` gates: keeping the enum shape aligned with
/// [`wakfu_log_path`]'s own per-OS branches avoids dead-code warnings when
/// a single-platform build only compiles one branch.
#[derive(Debug, thiserror::Error)]
pub enum LogPathError {
    #[cfg(target_os = "windows")]
    #[error("APPDATA environment variable is not set")]
    AppDataNotSet,
    #[cfg(target_os = "macos")]
    #[error("HOME environment variable is not set")]
    HomeNotSet,
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[error("unsupported OS for wakfu log path")]
    UnsupportedOs,
}

pub fn wakfu_log_path() -> Result<PathBuf, LogPathError> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").map_err(|_| LogPathError::AppDataNotSet)?;
        Ok(PathBuf::from(appdata)
            .join("zaap")
            .join("gamesLogs")
            .join("wakfu")
            .join("logs")
            .join("wakfu.log"))
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| LogPathError::HomeNotSet)?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("zaap")
            .join("wakfu")
            .join("logs")
            .join("wakfu.log"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(LogPathError::UnsupportedOs)
    }
}

// All current coverage exercises the macOS branch of `wakfu_log_path`; the
// whole module is gated so it doesn't sit empty (and its `use super::*`
// doesn't go unused) on other targets.
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn home_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn resolves_log_path_under_home_library_logs() {
        let _guard = home_env_lock().lock().unwrap();
        let previous = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", "/tmp/wakfu_dptmeter_fake_home");
        }

        let path = wakfu_log_path().unwrap();

        unsafe {
            match &previous {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert_eq!(
            path,
            PathBuf::from("/tmp/wakfu_dptmeter_fake_home/Library/Logs/zaap/wakfu/logs/wakfu.log")
        );
    }

    #[test]
    fn errors_when_home_is_not_set() {
        let _guard = home_env_lock().lock().unwrap();
        let previous = std::env::var("HOME").ok();
        unsafe {
            std::env::remove_var("HOME");
        }

        let result = wakfu_log_path();

        unsafe {
            if let Some(value) = &previous {
                std::env::set_var("HOME", value);
            }
        }

        assert!(result.is_err());
    }
}
