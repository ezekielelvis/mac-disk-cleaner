use std::path::Path;

#[inline(always)]
pub fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

/// Check if path is a system or critical file/folder
/// These files are visible but protected from accidental deletion
#[inline(always)]
pub fn is_system_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    // macOS System directories
    path_str.starts_with("/System") ||
    path_str.starts_with("/usr") ||
    path_str.starts_with("/bin") ||
    path_str.starts_with("/sbin") ||
    path_str.starts_with("/private/var") ||
    path_str.starts_with("/private/etc") ||
    
    // Library system data
    path_str.contains("/Library/Keychains") ||
    path_str.contains("/Library/Application Support/com.apple") ||
    path_str.contains("/Library/Preferences/com.apple") ||
    path_str.contains("/Library/Containers") ||
    path_str.contains("/Library/Group Containers") ||
    path_str.contains("/Library/LaunchAgents") ||
    path_str.contains("/Library/LaunchDaemons") ||
    
    // User config files
    path_str.contains("/.ssh") ||
    path_str.contains("/.gnupg") ||
    path_str.contains("/.config") ||
    path_str.contains("/.local") ||
    path_str.ends_with(".zshrc") ||
    path_str.ends_with(".bashrc") ||
    path_str.ends_with(".bash_profile") ||
    path_str.ends_with(".zsh_history") ||
    path_str.ends_with(".bash_history") ||
    
    // Critical app data
    path_str.contains("/CoreServices") ||
    path_str.contains("/PrivateFrameworks") ||
    path_str.contains("/Frameworks")
}

pub fn get_system_warning(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy();
    
    // Security-critical warnings
    if path_str.contains("Keychains") {
        return Some("⚠️ Contains encryption keys and passwords!".to_string());
    }
    if path_str.contains(".ssh") {
        return Some("⚠️ SSH authentication keys!".to_string());
    }
    if path_str.contains(".gnupg") {
        return Some("⚠️ GPG encryption keys!".to_string());
    }
    
    // System-critical warnings
    if path_str.starts_with("/System") {
        return Some("🛑 CORE SYSTEM FILES - DO NOT DELETE!".to_string());
    }
    if path_str.starts_with("/usr") || path_str.starts_with("/bin") || path_str.starts_with("/sbin") {
        return Some("🛑 UNIX SYSTEM BINARIES - DO NOT DELETE!".to_string());
    }
    if path_str.starts_with("/private/var") {
        return Some("⚠️ System runtime data".to_string());
    }
    if path_str.starts_with("/private/etc") {
        return Some("🛑 System configuration files!".to_string());
    }
    
    // Application data warnings
    if path_str.contains("/Library/LaunchAgents") || path_str.contains("/Library/LaunchDaemons") {
        return Some("⚠️ System/app startup scripts".to_string());
    }
    if path_str.contains("/Library/Containers") {
        return Some("⚠️ App sandbox data - may break apps".to_string());
    }
    if path_str.contains("/Library/Group Containers") {
        return Some("⚠️ Shared app data - may break apps".to_string());
    }
    if path_str.contains("/Library/Application Support/com.apple") {
        return Some("⚠️ Apple system app data".to_string());
    }
    if path_str.contains("CoreServices") || path_str.contains("Frameworks") {
        return Some("🛑 System frameworks - DO NOT DELETE!".to_string());
    }
    
    // Config file warnings
    if path_str.ends_with(".zshrc") || path_str.ends_with(".bashrc") || path_str.ends_with(".bash_profile") {
        return Some("⚠️ Shell configuration file".to_string());
    }
    if path_str.ends_with(".zsh_history") || path_str.ends_with(".bash_history") {
        return Some("⚠️ Command history - contains past commands".to_string());
    }
    if path_str.contains("/.config") {
        return Some("⚠️ App configuration directory".to_string());
    }
    
    None
}
