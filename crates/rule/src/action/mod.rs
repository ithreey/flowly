#[cfg(feature = "js")]
pub mod js;
mod log;
mod modify;

pub use self::log::*;
pub use modify::{Modify, is_textual_content_type};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Reject,
    Redirect(String),
    ModifyRequest(Modify),
    ModifyResponse(Modify),
    LogRes,
    LogReq,
    /// 交互拦截：命中时由 GUI 层弹出决策窗口，规则执行本身忽略此动作。
    Intercept,

    #[cfg(feature = "js")]
    Js(String),
}
