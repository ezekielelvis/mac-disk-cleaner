use std::path::Path;

#[inline(always)]
pub fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

#[inline(always)]
pub fn is_system_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    path_str.starts_with("/System") ||
    path_str.starts_with("/usr") ||
    path_str.starts_with("/bin") ||
    path_str.starts_with("/sbin") ||
    path_str.contains("/Library/Keychains") ||
    path_str.contains("/.ssh") ||
    path_str.contains("/.gnupg") ||
    path_str.ends_with(".zshrc") ||
    path_str.ends_with(".bashrc") ||
    path_str.ends_with(".bash_profile")
}

pub fn get_system_warning(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    
    if path_str.contains("Keychains") {
        return Some("⚠️ Contains encryption keys!".to_string());
    }
    if path_str.contains(".ssh") {
        return Some("⚠️ SSH keys!".to_string());
    }
    if path_str.contains(".gnupg") {
        return Some("⚠️ GPG keys!".to_string());
    }
    if path_str.contains("/System") {
        return Some("🛑 SYSTEM FILES!".to_string());
    }
    None
}
