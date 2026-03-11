use std::env::current_exe;
use std::path::PathBuf;

/// Returns root path of current executable
/// For example if current executable is
/// under `/opt/onessa/onessa`, then this function will return `/opt/onessa`
/// ```rust
/// use onessa_utils::get_root_path;
///
/// let root_path = get_root_path();
/// let favicon_path = root_path.join("favicon.ico");
///
/// assert!(root_path.is_absolute());
/// assert!(root_path.is_dir());
///
/// assert!(favicon_path.is_absolute());
/// assert!(!favicon_path.is_dir());
/// ```
///
pub fn get_root_path() -> PathBuf {
    let cur_exe = current_exe().expect("Failed to find current executable");
    let cur_parent = cur_exe
        .parent()
        .expect("Failed to find current parent");
    cur_parent.to_path_buf()
}
