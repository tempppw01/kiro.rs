//! Kiro 模型目录。
//!
//! 模型名称会同时出现在 `/v1/models`、请求转换和上下文窗口判断中。
//! 集中维护可以避免新增模型时只改到其中一处。
//!
//! 模型名称和发布时间依据 Kiro 官方文档（2026-08-26 核对）；Kiro ID
//! 沿用现有请求中使用的命名规则，并集中放在 `kiro_id` 字段中。

/// 模型是否需要付费订阅。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelAccess {
    Free,
    Premium,
}

/// 一个对外模型及其对应的 Kiro 模型 ID。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelSpec {
    pub id: &'static str,
    pub kiro_id: &'static str,
    pub display_name: &'static str,
    pub created: i64,
    pub max_tokens: i32,
    pub context_window: i32,
    pub access: ModelAccess,
    pub supports_thinking: bool,
}

/// 模型目录按官方页面的发布时间倒序排列。
pub(crate) const MODEL_CATALOG: &[ModelSpec] = &[
    ModelSpec {
        id: "gpt-5.6-sol",
        kiro_id: "gpt-5.6-sol",
        display_name: "GPT-5.6 Sol",
        created: 1_783_900_800,
        max_tokens: 128_000,
        context_window: 200_000,
        access: ModelAccess::Premium,
        supports_thinking: false,
    },
    ModelSpec {
        id: "gpt-5.6-terra",
        kiro_id: "gpt-5.6-terra",
        display_name: "GPT-5.6 Terra",
        created: 1_783_900_800,
        max_tokens: 128_000,
        context_window: 200_000,
        access: ModelAccess::Premium,
        supports_thinking: false,
    },
    ModelSpec {
        id: "gpt-5.6-luna",
        kiro_id: "gpt-5.6-luna",
        display_name: "GPT-5.6 Luna",
        created: 1_783_900_800,
        max_tokens: 128_000,
        context_window: 200_000,
        access: ModelAccess::Premium,
        supports_thinking: false,
    },
    ModelSpec {
        id: "claude-opus-5",
        kiro_id: "claude-opus-5",
        display_name: "Claude Opus 5",
        created: 1_784_851_200,
        max_tokens: 128_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-sonnet-5",
        kiro_id: "claude-sonnet-5",
        display_name: "Claude Sonnet 5",
        created: 1_782_777_600,
        max_tokens: 128_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-opus-4-8",
        kiro_id: "claude-opus-4.8",
        display_name: "Claude Opus 4.8",
        created: 1_779_926_400,
        max_tokens: 128_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-opus-4-7",
        kiro_id: "claude-opus-4.7",
        display_name: "Claude Opus 4.7",
        created: 1_776_297_600,
        max_tokens: 64_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "glm-5",
        kiro_id: "glm-5",
        display_name: "GLM-5",
        created: 1_774_915_200,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "minimax-m2.5",
        kiro_id: "minimax-m2.5",
        display_name: "MiniMax M2.5",
        created: 1_773_792_000,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "claude-sonnet-4-6",
        kiro_id: "claude-sonnet-4.6",
        display_name: "Claude Sonnet 4.6",
        created: 1_771_286_400,
        max_tokens: 64_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "deepseek-3.2",
        kiro_id: "deepseek-3.2",
        display_name: "DeepSeek 3.2",
        created: 1_770_681_600,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "minimax-m2.1",
        kiro_id: "minimax-m2.1",
        display_name: "MiniMax M2.1",
        created: 1_770_681_600,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "qwen3-coder-next",
        kiro_id: "qwen3-coder-next",
        display_name: "Qwen3 Coder Next",
        created: 1_770_681_600,
        max_tokens: 64_000,
        context_window: 256_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "claude-opus-4-6",
        kiro_id: "claude-opus-4.6",
        display_name: "Claude Opus 4.6",
        created: 1_770_249_600,
        max_tokens: 64_000,
        context_window: 1_000_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-opus-4-5-20251101",
        kiro_id: "claude-opus-4.5",
        display_name: "Claude Opus 4.5",
        created: 1_763_942_400,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-sonnet-4-5-20250929",
        kiro_id: "claude-sonnet-4.5",
        display_name: "Claude Sonnet 4.5",
        created: 1_759_104_000,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: true,
    },
    ModelSpec {
        id: "auto",
        kiro_id: "auto",
        display_name: "Auto",
        created: 1_758_067_200,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: false,
    },
    ModelSpec {
        id: "claude-sonnet-4-20250514",
        kiro_id: "claude-sonnet-4.0",
        display_name: "Claude Sonnet 4.0",
        created: 1_756_944_000,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Free,
        supports_thinking: true,
    },
    ModelSpec {
        id: "claude-haiku-4-5-20251001",
        kiro_id: "claude-haiku-4.5",
        display_name: "Claude Haiku 4.5",
        created: 1_756_944_000,
        max_tokens: 64_000,
        context_window: 200_000,
        access: ModelAccess::Premium,
        supports_thinking: true,
    },
];

/// 返回带有指定 thinking 后缀的对外模型 ID。
pub(crate) fn thinking_id(spec: &ModelSpec) -> String {
    format!("{}-thinking", spec.id)
}

/// 根据对外名称解析模型。
///
/// 除了目录中的规范名称，也保留旧客户端常用的日期版本和点号/短横线写法。
pub(crate) fn resolve_model(model: &str) -> Option<&'static ModelSpec> {
    let model = model.trim().to_lowercase();
    let is_thinking = model.ends_with("-thinking");
    let base = model.strip_suffix("-thinking").unwrap_or(&model);

    if let Some(spec) = MODEL_CATALOG.iter().find(|spec| {
        spec.id == base
            || spec.kiro_id == base
            || (spec.id == "claude-sonnet-4-20250514" && base == "claude-sonnet-4-0")
            || (spec.id == "deepseek-3.2" && base == "deepseek-3-2")
            || (spec.id == "minimax-m2.5" && base == "minimax-m2-5")
            || (spec.id == "minimax-m2.1" && base == "minimax-m2-1")
    }) {
        return (spec.supports_thinking || !is_thinking).then_some(spec);
    }

    // Anthropic 客户端通常发送带日期的模型名；将旧版本归入 Kiro 中相应的兼容模型。
    if base.contains("sonnet") {
        if base.contains("sonnet-5") {
            return MODEL_CATALOG.iter().find(|s| s.id == "claude-sonnet-5");
        }
        if base.contains("4-6") || base.contains("4.6") {
            return MODEL_CATALOG.iter().find(|s| s.id == "claude-sonnet-4-6");
        }
        if base.contains("4-0")
            || base.contains("4.0")
            || base.ends_with("sonnet-4")
            || (base.contains("sonnet-4-") && !base.contains("4-5"))
        {
            return MODEL_CATALOG
                .iter()
                .find(|s| s.id == "claude-sonnet-4-20250514");
        }
        return MODEL_CATALOG
            .iter()
            .find(|s| s.id == "claude-sonnet-4-5-20250929");
    }

    if base.contains("opus") {
        if base.contains("opus-5") {
            return MODEL_CATALOG.iter().find(|s| s.id == "claude-opus-5");
        }
        if base.contains("4-8") || base.contains("4.8") {
            return MODEL_CATALOG.iter().find(|s| s.id == "claude-opus-4-8");
        }
        if base.contains("4-7") || base.contains("4.7") {
            return MODEL_CATALOG.iter().find(|s| s.id == "claude-opus-4-7");
        }
        if base.contains("4-5") || base.contains("4.5") {
            return MODEL_CATALOG
                .iter()
                .find(|s| s.id == "claude-opus-4-5-20251101");
        }
        return MODEL_CATALOG.iter().find(|s| s.id == "claude-opus-4-6");
    }

    if base.contains("haiku") {
        return MODEL_CATALOG
            .iter()
            .find(|s| s.id == "claude-haiku-4-5-20251001");
    }

    None
}

pub(crate) fn map_model(model: &str) -> Option<String> {
    resolve_model(model).map(|spec| spec.kiro_id.to_string())
}

pub(crate) fn context_window_size(model: &str) -> i32 {
    resolve_model(model)
        .map(|spec| spec.context_window)
        .unwrap_or(200_000)
}

pub(crate) fn requires_paid_subscription(model: &str) -> bool {
    resolve_model(model)
        .map(|spec| spec.access == ModelAccess::Premium)
        .unwrap_or(false)
}

/// 将目录转换成 Anthropic `/v1/models` 响应。
pub(crate) fn api_models<F>(mut supports_model: F) -> Vec<super::types::Model>
where
    F: FnMut(&str) -> bool,
{
    MODEL_CATALOG
        .iter()
        .filter(|spec| supports_model(spec.kiro_id))
        .flat_map(|spec| {
            let base = super::types::Model {
                id: spec.id.to_string(),
                object: "model".to_string(),
                created: spec.created,
                // 保留既有 Claude 模型的响应字段，开放权重/GPT 模型归 Kiro。
                owned_by: (if spec.id.starts_with("claude-") {
                    "anthropic"
                } else {
                    "kiro"
                })
                .to_string(),
                display_name: spec.display_name.to_string(),
                model_type: "chat".to_string(),
                max_tokens: spec.max_tokens,
            };

            if spec.supports_thinking {
                vec![
                    base.clone(),
                    super::types::Model {
                        id: thinking_id(spec),
                        display_name: format!("{} (Thinking)", spec.display_name),
                        ..base
                    },
                ]
            } else {
                vec![base]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_catalog_contains_the_models_from_kiro_docs() {
        let free: Vec<_> = MODEL_CATALOG
            .iter()
            .filter(|spec| spec.access == ModelAccess::Free)
            .map(|spec| spec.id)
            .collect();

        for id in [
            "auto",
            "claude-sonnet-4-5-20250929",
            "claude-sonnet-4-20250514",
            "deepseek-3.2",
            "minimax-m2.5",
            "glm-5",
            "minimax-m2.1",
            "qwen3-coder-next",
        ] {
            assert!(free.contains(&id), "missing Free model: {id}");
        }
    }

    #[test]
    fn aliases_and_thinking_suffix_resolve_to_the_same_kiro_id() {
        assert_eq!(map_model("claude-sonnet-4-20250514"), Some("claude-sonnet-4.0".into()));
        assert_eq!(map_model("claude-sonnet-4-5-20250929-thinking"), Some("claude-sonnet-4.5".into()));
        assert_eq!(map_model("claude-opus-4.8-thinking"), Some("claude-opus-4.8".into()));
    }

    #[test]
    fn open_models_do_not_require_thinking_suffixes() {
        assert!(!resolve_model("deepseek-3.2-thinking").is_some_and(|s| s.supports_thinking));
        assert_eq!(context_window_size("qwen3-coder-next"), 256_000);
    }

    #[test]
    fn api_models_can_be_filtered_to_free_models() {
        let models = api_models(|id| !requires_paid_subscription(id));
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();

        assert!(ids.contains(&"auto"));
        assert!(ids.contains(&"deepseek-3.2"));
        assert!(ids.contains(&"claude-sonnet-4-5-20250929-thinking"));
        assert!(!ids.iter().any(|id| id.contains("opus")));
        assert!(!ids.contains(&"claude-sonnet-5"));
    }
}
