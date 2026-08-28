use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

const MAX_HISTORY: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValueRow {
    pub key: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: u64,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValueRow>,
    #[serde(default)]
    pub form_rows: Vec<KeyValueRow>,
    pub headers: Vec<(String, String)>,
    #[serde(default = "default_body_type")]
    pub body_type: String,
    #[serde(default = "default_body_raw_format")]
    pub body_raw_format: String,
    pub body: Option<Vec<u8>>,
    pub timestamp: i64,
}

fn default_enabled() -> bool {
    true
}

fn default_body_type() -> String {
    "none".to_string()
}

fn default_body_raw_format() -> String {
    "Text".to_string()
}

pub struct HistoryStore {
    entries: Mutex<Vec<HistoryEntry>>,
    next_id: Mutex<u64>,
}

impl HistoryStore {
    pub fn new(path: &Path) -> Self {
        let entries = load_history(path);
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self {
            entries: Mutex::new(entries),
            next_id: Mutex::new(next_id),
        }
    }

    pub fn list(&self) -> Vec<HistoryEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn save(&self, mut entry: HistoryEntry) -> HistoryEntry {
        let mut entries = self.entries.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        entry.id = *next_id;
        *next_id += 1;
        entries.insert(0, entry.clone());
        if entries.len() > MAX_HISTORY {
            entries.truncate(MAX_HISTORY);
        }
        entry
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    pub fn delete(&self, id: u64) {
        self.entries.lock().unwrap().retain(|e| e.id != id);
    }

    pub fn persist(&self, path: &Path) -> Result<(), String> {
        let entries = self.entries.lock().unwrap();
        let json = serde_json::to_string_pretty(&*entries)
            .map_err(|e| format!("序列化历史记录失败: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("写历史记录失败: {e}"))?;
        Ok(())
    }
}

fn load_history(path: &Path) -> Vec<HistoryEntry> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

#[tauri::command]
pub fn history_list(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<HistoryEntry>, String> {
    Ok(state.history.list())
}

#[tauri::command]
pub fn history_save(
    state: tauri::State<'_, crate::state::AppState>,
    entry: HistoryEntry,
) -> Result<HistoryEntry, String> {
    let saved = state.history.save(entry);
    state.history.persist(&state.history_path)?;
    Ok(saved)
}

#[tauri::command]
pub fn history_clear(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<(), String> {
    state.history.clear();
    state.history.persist(&state.history_path)?;
    Ok(())
}

#[tauri::command]
pub fn history_delete(
    state: tauri::State<'_, crate::state::AppState>,
    id: u64,
) -> Result<(), String> {
    state.history.delete(id);
    state.history.persist(&state.history_path)?;
    Ok(())
}
