//! 规则配置管理命令：以「每条规则一个 JSON 字符串」为单位。
//!
//! 前端编辑的是单条规则的 JSON 文本，后端负责解析校验、转换、
//! 热应用（`set_rules` + `set_filters`）并写回规则文件。

use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// 前端规则条目：name/enabled 用于列表展示与开关，json 是单条规则的 JSON 文本（顶层对象）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleEntry {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub json: String,
}

fn default_enabled() -> bool {
    true
}

type FRule = flowly::file::frule::Rule;

/// 读取当前规则文件，返回每条规则的 name + json。
#[tauri::command]
pub fn rules_list(state: tauri::State<'_, AppState>) -> Result<Vec<RuleEntry>, String> {
    let content =
        std::fs::read_to_string(&state.rules_path).map_err(|e| format!("读取规则文件失败: {e}"))?;
    let rules: Vec<FRule> =
        serde_json::from_str(&content).map_err(|e| format!("解析规则失败: {e}"))?;
    rules
        .into_iter()
        .map(|r| {
            let name = r.name.clone();
            let enabled = r.enabled;
            let json =
                serde_json::to_string_pretty(&r).map_err(|e| format!("序列化规则失败: {e}"))?;
            Ok(RuleEntry {
                name,
                enabled,
                json,
            })
        })
        .collect()
}

/// 保存并热应用全部规则：解析每条 JSON → 转换 → `handler.set_rules` +
/// `mitm_filter.set_filters` → 写回规则/配置文件。返回生效规则条数。
#[tauri::command]
pub fn rules_save(
    state: tauri::State<'_, AppState>,
    entries: Vec<RuleEntry>,
    mitm_hosts: Vec<String>,
) -> Result<usize, String> {
    let mut frules: Vec<FRule> = Vec::with_capacity(entries.len());
    let mut exec_rules = vec![];

    for (i, entry) in entries.iter().enumerate() {
        let fr: FRule = serde_json::from_str(&entry.json)
            .map_err(|e| format!("第 {} 条规则解析失败: {e}", i + 1))?;
        let (r, _legacy_mitm_filters): (rule::Rule, Vec<String>) = fr.clone().into();
        exec_rules.push(r);
        frules.push(fr);
    }
    let mitm_hosts = normalize_mitm_hosts(mitm_hosts);

    // 热应用：无需重启代理，正在进行的连接继续用旧快照，新请求用新规则。
    state.handler.set_rules(exec_rules);
    state.mitm_filter.set_filters(mitm_hosts.clone());

    // 写回规则文件（美化 JSON）。
    let doc = serde_json::to_string_pretty(&frules).map_err(|e| format!("序列化规则失败: {e}"))?;
    std::fs::write(&state.rules_path, doc).map_err(|e| format!("写规则文件失败: {e}"))?;

    let mut config = state.config.write().unwrap();
    config.mitm_hosts = mitm_hosts;
    crate::config::save_config(&state.config_path, &config)?;

    Ok(frules.len())
}

/// 导入 JSON：解析完整文档（数组）或单条规则（对象），返回条目列表供前端替换。
#[tauri::command]
pub fn rules_import_json(json: String) -> Result<Vec<RuleEntry>, String> {
    let rules = parse_json(&json)?;
    rules
        .into_iter()
        .map(|r| {
            let name = r.name.clone();
            let enabled = r.enabled;
            let json =
                serde_json::to_string_pretty(&r).map_err(|e| format!("序列化规则失败: {e}"))?;
            Ok(RuleEntry {
                name,
                enabled,
                json,
            })
        })
        .collect()
}

fn parse_json(json: &str) -> Result<Vec<FRule>, String> {
    // 先按规则数组解析。
    if let Ok(rules) = serde_json::from_str::<Vec<FRule>>(json) {
        return Ok(rules);
    }
    // 再按单条规则对象解析。
    let single: FRule = serde_json::from_str(json).map_err(|e| format!("JSON 解析失败: {e}"))?;
    Ok(vec![single])
}

fn normalize_mitm_hosts(hosts: Vec<String>) -> Vec<String> {
    hosts
        .into_iter()
        .map(|host| host.trim().to_string())
        .filter(|host| !host.is_empty())
        .collect()
}
