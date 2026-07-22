mod log_locator;
mod log_tailer;
mod log_watcher;

pub use log_locator::get_path;
pub use log_watcher::watch;
