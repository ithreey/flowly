use serde::{Deserialize, Serialize};

use super::SingleOrMulti;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub name: String,
    /// 启用/禁用开关，disabled 的规则不参与匹配与 MITM。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(alias = "mitm")]
    pub mitm_list: Option<SingleOrMulti<String>>,
    #[serde(alias = "filter")]
    pub filters: SingleOrMulti<rule::Filter>,
    #[serde(alias = "action")]
    pub actions: SingleOrMulti<rule::Action>,
}

fn default_enabled() -> bool {
    true
}

impl From<Rule> for (rule::Rule, Vec<String>) {
    fn from(rule: Rule) -> Self {
        // 禁用的规则转换为空规则：不匹配任何请求、不贡献 MITM 域名。
        if !rule.enabled {
            return (
                rule::Rule {
                    filters: vec![],
                    actions: vec![],
                    url: None,
                },
                vec![],
            );
        }

        let filters: Vec<rule::Filter> = rule
            .filters
            .into_vec()
            .iter()
            .map(rule::Filter::init)
            .collect();

        // 显式声明 MITM 域名时优先使用，不再叠加 filter 推导——
        // 否则 `filter: all` 推导的 "*" 会把显式 mitmList 限制覆盖成全量 MITM。
        let mut mitm_filters: Vec<String> = if rule.mitm_list.is_some() {
            vec![]
        } else {
            filters
                .iter()
                .filter_map(rule::Filter::mitm_filtter_pattern)
                .collect()
        };

        if let Some(s) = rule.mitm_list {
            mitm_filters.append(&mut s.into_vec());
        }

        let rule = rule::Rule {
            filters,
            actions: rule.actions.into_vec(),
            url: None,
        };

        (rule, mitm_filters)
    }
}
