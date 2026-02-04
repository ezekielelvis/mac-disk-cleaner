mod scanner;

pub use scanner::Scanner;

use std::path::Path;

/// Get warning message for critical system paths
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
