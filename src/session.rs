use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub content: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Session {
    pub fn new(content: String) -> Self {
        let now = now_secs();
        Session { content, created_at: now, updated_at: now }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn dirs_fallback(relative: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(relative)
    } else {
        PathBuf::from(relative)
    }
}

fn data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs_fallback("Library/Application Support/numnum")
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("numnum")
        } else {
            dirs_fallback(".local/share/numnum")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            PathBuf::from(xdg).join("numnum")
        } else {
            dirs_fallback(".local/share/numnum")
        }
    }
}

pub fn sessions_dir() -> PathBuf {
    let dir = data_dir().join("sessions");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn new_session_path() -> PathBuf {
    let dir = sessions_dir();
    let timestamp = now_secs();
    let base = format!("numnum_session_{}", timestamp);
    let mut path = dir.join(format!("{}.json", base));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{}{}.json", base, n));
        n += 1;
    }
    path
}

pub fn list_sessions() -> Vec<(PathBuf, Session)> {
    let dir = sessions_dir();
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        if !session.content.trim().is_empty() {
                            sessions.push((path, session));
                        }
                    }
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
    sessions
}

pub fn save_session(path: &Path, session: &Session) {
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let _ = fs::write(path, json);
    }
}

pub fn load_session(path: &Path) -> Option<Session> {
    fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn format_display_name(content: &str) -> String {
    let first = content
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty());
    match first {
        None => "<empty session>".to_string(),
        Some(line) if line.len() <= 22 => line.to_string(),
        Some(line) => format!("{}...", &line[..22]),
    }
}

pub fn format_timestamp(secs: u64) -> String {
    let now = now_secs();
    let delta = now.saturating_sub(secs);
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3600 {
        format!("{} min ago", delta / 60)
    } else if delta < 86400 {
        format!("{} hr ago", delta / 3600)
    } else {
        format!("{} days ago", delta / 86400)
    }
}

// ── Internal helpers for testability ───────────────────────────────────────

fn new_session_path_in_dir(dir: &Path) -> PathBuf {
    let timestamp = now_secs();
    let base = format!("numnum_session_{}", timestamp);
    let mut path = dir.join(format!("{}.json", base));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{}{}.json", base, n));
        n += 1;
    }
    path
}

fn list_sessions_in_dir(dir: &Path) -> Vec<(PathBuf, Session)> {
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        if !session.content.trim().is_empty() {
                            sessions.push((path, session));
                        }
                    }
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
    sessions
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join(format!("numnum_session_test_{}_{}", std::process::id(), n));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // Simulates App::save_current_session logic for testing flows.
    fn simulate_save(
        session_path: &mut Option<PathBuf>,
        content: &str,
        dir: &Path,
    ) -> Option<PathBuf> {
        if content.trim().is_empty() {
            if let Some(path) = session_path.take() {
                let _ = fs::remove_file(&path);
            }
            None
        } else {
            let is_new = session_path.is_none();
            let path = match session_path {
                Some(p) => p.clone(),
                None => {
                    let p = new_session_path_in_dir(dir);
                    *session_path = Some(p.clone());
                    p
                }
            };
            let mut session = load_session(&path)
                .unwrap_or_else(|| Session::new(content.to_string()));
            // Always save new files. For existing files, only save when content
            // changed so timestamp reflects actual edit time, not reopen time.
            if is_new || session.content != content {
                session.content = content.to_string();
                session.updated_at = now_secs();
                save_session(&path, &session);
            }
            Some(path)
        }
    }

    // Simulates App::switch_session: save current, load other.
    fn simulate_switch(
        current_path: &mut Option<PathBuf>,
        current_content: &str,
        target_path: &Path,
        dir: &Path,
    ) -> Option<Session> {
        let _ = simulate_save(current_path, current_content, dir);
        load_session(target_path)
    }

    // Simulates App::new_session: save current (or delete if empty), clear.
    fn simulate_new_session(
        session_path: &mut Option<PathBuf>,
        current_content: &str,
        dir: &Path,
    ) {
        let _ = simulate_save(session_path, current_content, dir);
        *session_path = None;
    }

    #[test]
    fn test_new_session_path_unique() {
        let dir = test_dir();
        let a = new_session_path_in_dir(&dir);
        // Create file so next call collides and picks a different name.
        fs::write(&a, "{}").unwrap();
        let b = new_session_path_in_dir(&dir);
        assert_ne!(a, b);
        cleanup(&dir);
    }

    #[test]
    fn test_new_session_path_collision() {
        let dir = test_dir();
        let first = new_session_path_in_dir(&dir);
        fs::write(&first, "{}").unwrap();
        let second = new_session_path_in_dir(&dir);
        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with("1.json"));
        cleanup(&dir);
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = test_dir();
        let path = dir.join("test.json");
        let session = Session {
            content: "2 + 2\n".to_string(),
            created_at: 1000,
            updated_at: 2000,
        };
        save_session(&path, &session);
        let loaded = load_session(&path).unwrap();
        assert_eq!(loaded.content, "2 + 2\n");
        assert_eq!(loaded.created_at, 1000);
        assert_eq!(loaded.updated_at, 2000);
        cleanup(&dir);
    }

    #[test]
    fn test_list_sessions_filters_empty() {
        let dir = test_dir();
        let empty = Session::new("   \n  \t  ".to_string());
        let non_empty = Session::new("total = 100".to_string());
        save_session(&dir.join("a.json"), &empty);
        save_session(&dir.join("b.json"), &non_empty);
        let list = list_sessions_in_dir(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1.content, "total = 100");
        cleanup(&dir);
    }

    #[test]
    fn test_list_sessions_sorts_by_updated_at() {
        let dir = test_dir();
        let older = Session {
            content: "older".to_string(),
            created_at: 100,
            updated_at: 100,
        };
        let newer = Session {
            content: "newer".to_string(),
            created_at: 200,
            updated_at: 500,
        };
        save_session(&dir.join("older.json"), &older);
        save_session(&dir.join("newer.json"), &newer);
        let list = list_sessions_in_dir(&dir);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].1.content, "newer");
        assert_eq!(list[1].1.content, "older");
        cleanup(&dir);
    }

    #[test]
    fn test_list_sessions_skips_corrupted() {
        let dir = test_dir();
        let valid = Session::new("valid".to_string());
        save_session(&dir.join("good.json"), &valid);
        fs::write(&dir.join("bad.json"), "not json {{").unwrap();
        let list = list_sessions_in_dir(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1.content, "valid");
        cleanup(&dir);
    }

    #[test]
    fn test_list_sessions_ignores_non_json() {
        let dir = test_dir();
        let s = Session::new("hello".to_string());
        save_session(&dir.join("hello.json"), &s);
        fs::write(&dir.join("readme.txt"), "text").unwrap();
        let list = list_sessions_in_dir(&dir);
        assert_eq!(list.len(), 1);
        cleanup(&dir);
    }

    #[test]
    fn test_format_display_name_empty() {
        assert_eq!(format_display_name(""), "<empty session>");
    }

    #[test]
    fn test_format_display_name_whitespace_only() {
        assert_eq!(format_display_name("   \n\t  \n"), "<empty session>");
    }

    #[test]
    fn test_format_display_name_short_line() {
        assert_eq!(format_display_name("2 + 2"), "2 + 2");
    }

    #[test]
    fn test_format_display_name_long_line() {
        let long = "this is a very long first line of content";
        assert_eq!(format_display_name(long), "this is a very long fi...");
    }

    #[test]
    fn test_format_display_name_multiline_skips_blanks() {
        let text = "\n\n  \nactual content\nmore stuff";
        assert_eq!(format_display_name(text), "actual content");
    }

    #[test]
    fn test_format_display_name_exact_boundary() {
        let exact = "1234567890123456789012"; // 22 chars
        assert_eq!(format_display_name(exact), exact);
        let over = "12345678901234567890123"; // 23 chars
        assert_eq!(format_display_name(over), "1234567890123456789012...");
    }

    #[test]
    fn test_format_timestamp_just_now() {
        let now = now_secs();
        assert_eq!(format_timestamp(now), "just now");
        assert_eq!(format_timestamp(now - 30), "just now");
    }

    #[test]
    fn test_format_timestamp_minutes() {
        let now = now_secs();
        assert_eq!(format_timestamp(now - 120), "2 min ago");
        assert_eq!(format_timestamp(now - 59), "just now");
    }

    #[test]
    fn test_format_timestamp_hours() {
        let now = now_secs();
        assert_eq!(format_timestamp(now - 7200), "2 hr ago");
    }

    #[test]
    fn test_format_timestamp_days() {
        let now = now_secs();
        assert_eq!(format_timestamp(now - 172800), "2 days ago");
    }

    // ── Flow tests: simulate App-level session behavior ──────────────────────

    #[test]
    fn test_flow_type_then_clear_deletes_file() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // User types content: file created.
        let p = simulate_save(&mut path, "total = 42", &dir).unwrap();
        assert!(p.exists());
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        // User deletes everything: file deleted.
        let result = simulate_save(&mut path, "", &dir);
        assert!(result.is_none());
        assert!(path.is_none());
        assert!(!p.exists());
        assert_eq!(list_sessions_in_dir(&dir).len(), 0);

        cleanup(&dir);
    }

    #[test]
    fn test_flow_clear_then_type_creates_file() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // Type, then clear (deletes file).
        let first = simulate_save(&mut path, "hello", &dir).unwrap();
        simulate_save(&mut path, "", &dir);
        assert!(!first.exists());

        // Type again: file recreated. Path may be same or different depending
        // on timestamp rollover; what matters is the file exists with content.
        let second = simulate_save(&mut path, "world", &dir).unwrap();
        assert!(second.exists());
        let loaded = load_session(&second).unwrap();
        assert_eq!(loaded.content, "world");
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_flow_switch_sessions_saves_current() {
        let dir = test_dir();
        let mut path_a: Option<PathBuf> = None;

        // Build session A.
        let pa = simulate_save(&mut path_a, "session A content", &dir).unwrap();

        // Build session B separately.
        let mut path_b: Option<PathBuf> = None;
        let pb = simulate_save(&mut path_b, "session B content", &dir).unwrap();

        // Switch from A to B: A should be saved with latest content.
        let loaded = simulate_switch(&mut path_a, "updated A", &pb, &dir).unwrap();
        assert_eq!(loaded.content, "session B content");

        // Verify A was saved with updated content.
        let a_saved = load_session(&pa).unwrap();
        assert_eq!(a_saved.content, "updated A");

        cleanup(&dir);
    }

    #[test]
    fn test_flow_switch_from_empty_nothing_to_save() {
        let dir = test_dir();
        let mut path_a: Option<PathBuf> = None;

        // Session A is empty (no file on disk).
        assert!(path_a.is_none());

        // Build session B.
        let mut path_b: Option<PathBuf> = None;
        let pb = simulate_save(&mut path_b, "session B", &dir).unwrap();

        // Switch from empty A to B.
        let loaded = simulate_switch(&mut path_a, "", &pb, &dir).unwrap();
        assert_eq!(loaded.content, "session B");

        // No file should have been created for A.
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_flow_new_session_from_non_empty_saves_current() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // User has content.
        let original = simulate_save(&mut path, "original work", &dir).unwrap();

        // Click New Session: current saved, then cleared.
        simulate_new_session(&mut path, "original work", &dir);
        assert!(path.is_none());

        // Original file still exists with content.
        assert!(original.exists());
        let s = load_session(&original).unwrap();
        assert_eq!(s.content, "original work");

        // Editor is empty (no new file yet).
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_flow_new_session_from_empty_no_file_created() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // Editor is empty, no file exists.
        assert!(path.is_none());

        // Click New Session.
        simulate_new_session(&mut path, "", &dir);
        assert!(path.is_none());

        // Still no files on disk.
        assert_eq!(list_sessions_in_dir(&dir).len(), 0);

        cleanup(&dir);
    }

    #[test]
    fn test_flow_undo_after_clear_restores_and_creates_file() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // User types, then clears everything (file deleted).
        let first = simulate_save(&mut path, "important calculation", &dir).unwrap();
        simulate_save(&mut path, "", &dir);
        assert!(!first.exists());

        // Ctrl+Z restores content (simulated by typing again).
        // A new file is created; path may differ if timestamp rolled over.
        let second = simulate_save(&mut path, "important calculation", &dir).unwrap();
        assert!(second.exists());
        let loaded = load_session(&second).unwrap();
        assert_eq!(loaded.content, "important calculation");

        cleanup(&dir);
    }

    #[test]
    fn test_flow_partial_delete_then_undo_keeps_same_file() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // User has content, deletes a small part (content still non-empty).
        let p = simulate_save(&mut path, "total = 100 + 200", &dir).unwrap();
        let same = simulate_save(&mut path, "total = 100", &dir).unwrap();
        assert_eq!(p, same); // same file updated

        // Ctrl+Z restores (content back to non-empty).
        let still_same = simulate_save(&mut path, "total = 100 + 200", &dir).unwrap();
        assert_eq!(p, still_same);
        assert!(still_same.exists());

        cleanup(&dir);
    }

    #[test]
    fn test_created_at_preserved_across_updates() {
        let dir = test_dir();
        let path = dir.join("persist.json");
        let mut session = Session {
            content: "initial".to_string(),
            created_at: 42,
            updated_at: 42,
        };
        save_session(&path, &session);

        // Update content.
        session.content = "updated".to_string();
        session.updated_at = 99;
        save_session(&path, &session);

        let loaded = load_session(&path).unwrap();
        assert_eq!(loaded.created_at, 42); // preserved
        assert_eq!(loaded.updated_at, 99); // changed
        assert_eq!(loaded.content, "updated");

        cleanup(&dir);
    }

    #[test]
    fn test_concurrent_collision_handled() {
        let dir = test_dir();
        // Simulate two instances launching in the same second.
        let a = new_session_path_in_dir(&dir);
        fs::write(&a, "{}").unwrap();
        let b = new_session_path_in_dir(&dir);
        assert_ne!(a, b);
        assert!(b.to_string_lossy().contains("numnum_session_"));
        cleanup(&dir);
    }

    #[test]
    fn test_timestamp_not_updated_on_identical_save() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        // Initial save: content changes, timestamp set.
        let p = simulate_save(&mut path, "2 + 2", &dir).unwrap();
        let first = load_session(&p).unwrap();
        assert_eq!(first.content, "2 + 2");

        // Sleep to ensure timestamp would differ if updated.
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Re-save identical content: timestamp must stay the same.
        let _ = simulate_save(&mut path, "2 + 2", &dir).unwrap();
        let second = load_session(&p).unwrap();
        assert_eq!(second.content, "2 + 2");
        assert_eq!(second.updated_at, first.updated_at);

        cleanup(&dir);
    }

    // Simulates App::delete_session for testing.
    fn simulate_delete(
        current_path: &Option<PathBuf>,
        path_to_delete: &PathBuf,
        _dir: &Path,
    ) -> bool {
        if current_path.as_ref() == Some(path_to_delete) {
            return false;
        }
        let _ = fs::remove_file(path_to_delete);
        true
    }

    #[test]
    fn test_delete_non_current_removes_file() {
        let dir = test_dir();
        let mut path_a: Option<PathBuf> = None;
        let mut path_b: Option<PathBuf> = None;

        let pa = simulate_save(&mut path_a, "session A", &dir).unwrap();
        let pb = simulate_save(&mut path_b, "session B", &dir).unwrap();

        // Current session is A, delete B.
        let deleted = simulate_delete(&path_a, &pb, &dir);
        assert!(deleted);
        assert!(!pb.exists());
        assert!(pa.exists());
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_delete_current_is_guarded() {
        let dir = test_dir();
        let mut path: Option<PathBuf> = None;

        let p = simulate_save(&mut path, "current session", &dir).unwrap();

        // Attempt to delete current session.
        let deleted = simulate_delete(&path, &p, &dir);
        assert!(!deleted);
        assert!(p.exists());
        assert_eq!(list_sessions_in_dir(&dir).len(), 1);

        cleanup(&dir);
    }

    #[test]
    fn test_delete_updates_list() {
        let dir = test_dir();
        let mut path_a: Option<PathBuf> = None;
        let mut path_b: Option<PathBuf> = None;
        let mut path_c: Option<PathBuf> = None;

        let pa = simulate_save(&mut path_a, "A", &dir).unwrap();
        let _pb = simulate_save(&mut path_b, "B", &dir).unwrap();
        let pc = simulate_save(&mut path_c, "C", &dir).unwrap();

        // Current is A, delete C.
        let _ = simulate_delete(&path_a, &pc, &dir);

        let list = list_sessions_in_dir(&dir);
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|(p, _)| p == &pa));
        assert!(!list.iter().any(|(p, _)| p == &pc));

        cleanup(&dir);
    }
}
