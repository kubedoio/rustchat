//! Template Renderer Service
//!
//! Provides template rendering using Handlebars with strict variable resolution.
//! Supports MJML compilation and localization.

use handlebars::Handlebars;
use serde::Serialize;
use tracing::warn;

use crate::models::email::{EmailTemplateVersion, TemplateVariable};

/// Errors that can occur during template rendering
#[derive(Debug, Clone)]
pub enum TemplateError {
    RenderError(String),
    MissingVariable(String),
    InvalidTemplate(String),
    JsonError(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::RenderError(msg) => write!(f, "Render error: {}", msg),
            TemplateError::MissingVariable(name) => {
                write!(f, "Missing required variable: {}", name)
            }
            TemplateError::InvalidTemplate(msg) => write!(f, "Invalid template: {}", msg),
            TemplateError::JsonError(msg) => write!(f, "JSON error: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Result type for template operations
pub type TemplateResult<T> = Result<T, TemplateError>;

/// Context for template rendering
#[derive(Debug, Clone, Default)]
pub struct RenderContext {
    variables: serde_json::Value,
    strict_mode: bool,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            variables: serde_json::json!({}),
            strict_mode: true,
        }
    }

    pub fn with_variables<T: Serialize>(mut self, variables: T) -> TemplateResult<Self> {
        self.variables =
            serde_json::to_value(variables).map_err(|e| TemplateError::JsonError(e.to_string()))?;
        Ok(self)
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Validate that all required variables are present
    pub fn validate_required(&self, schema: &[TemplateVariable]) -> TemplateResult<()> {
        for var in schema {
            if var.required && self.get_variable(&var.name).is_none() {
                // Check if there's a default value
                if var.default_value.is_none() {
                    return Err(TemplateError::MissingVariable(var.name.clone()));
                }
            }
        }
        Ok(())
    }
}

/// Rendered email content
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
}

/// Template renderer using Handlebars
pub struct TemplateRenderer {
    handlebars: Handlebars<'static>,
    mjml_enabled: bool,
}

impl TemplateRenderer {
    /// Create a new template renderer
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        // Configure Handlebars for strict mode
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(handlebars::no_escape);

        // Register built-in helpers
        Self::register_helpers(&mut handlebars);

        Self {
            handlebars,
            mjml_enabled: false, // MJML compilation would require additional setup
        }
    }

    /// Create a new template renderer with MJML support
    pub fn with_mjml() -> Self {
        let mut renderer = Self::new();
        renderer.mjml_enabled = true;
        renderer
    }

    /// Register built-in Handlebars helpers
    fn register_helpers(handlebars: &mut Handlebars) {
        // eq helper for comparisons
        handlebars.register_helper(
            "eq",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param0 = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    let param1 = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
                    out.write(if param0 == param1 { "true" } else { "" })?;
                    Ok(())
                },
            ),
        );

        // ne helper (not equal)
        handlebars.register_helper(
            "ne",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param0 = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    let param1 = h.param(1).and_then(|v| v.value().as_str()).unwrap_or("");
                    out.write(if param0 != param1 { "true" } else { "" })?;
                    Ok(())
                },
            ),
        );

        // upper helper
        handlebars.register_helper(
            "upper",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    out.write(&param.to_uppercase())?;
                    Ok(())
                },
            ),
        );

        // lower helper
        handlebars.register_helper(
            "lower",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let param = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    out.write(&param.to_lowercase())?;
                    Ok(())
                },
            ),
        );

        // truncate helper
        handlebars.register_helper(
            "truncate",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let text = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    let length =
                        h.param(1).and_then(|v| v.value().as_i64()).unwrap_or(100) as usize;
                    if text.len() > length {
                        // Never slice through a multi-byte UTF-8 character
                        // (e.g. emoji/CJK in chat excerpts) — that panics.
                        let mut end = length;
                        while !text.is_char_boundary(end) {
                            end -= 1;
                        }
                        out.write(&format!("{}...", &text[..end]))?;
                    } else {
                        out.write(text)?;
                    }
                    Ok(())
                },
            ),
        );

        // format_date helper (simple implementation)
        handlebars.register_helper(
            "format_date",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    // Simple date formatting - in production, use a proper date library
                    let date = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    out.write(date)?;
                    Ok(())
                },
            ),
        );
    }

    /// Render a template string with context
    pub fn render_template(
        &self,
        template: &str,
        context: &RenderContext,
    ) -> TemplateResult<String> {
        // Create a temporary template
        let template_name = "__temp__";

        // Register the template (this is inefficient for single renders, but works)
        let mut hb = self.handlebars.clone();
        hb.register_template_string(template_name, template)
            .map_err(|e| TemplateError::InvalidTemplate(e.to_string()))?;

        // Render
        let result = hb.render(template_name, &context.variables).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("helper") && msg.contains("not defined") {
                // Extract missing variable name from error
                let var_name = msg.split('"').nth(1).unwrap_or("unknown").to_string();
                TemplateError::MissingVariable(var_name)
            } else {
                TemplateError::RenderError(msg)
            }
        })?;

        Ok(result)
    }

    /// Render an email template with validation
    pub fn render_email(
        &self,
        template: &EmailTemplateVersion,
        context: &RenderContext,
    ) -> TemplateResult<RenderedEmail> {
        // Validate required variables if strict mode is on
        if context.strict_mode {
            context.validate_required(&template.variables_schema_json.0)?;
        }

        // Render subject
        let subject = self.render_template(&template.subject, context)?;

        // Render text body
        let body_text = self.render_template(&template.body_text, context)?;

        // Render HTML body if present
        let body_html = if template.body_html.is_empty() {
            None
        } else {
            Some(self.render_template(&template.body_html, context)?)
        };

        Ok(RenderedEmail {
            subject,
            body_text,
            body_html,
        })
    }

    /// Preview a template with sample data
    pub fn preview_template(
        &self,
        template: &EmailTemplateVersion,
        sample_data: &serde_json::Value,
    ) -> TemplateResult<RenderedEmail> {
        let context = RenderContext::new().with_variables(sample_data)?;

        self.render_email(template, &context)
    }

    /// Validate a template without rendering
    pub fn validate_template(&self, template: &str) -> TemplateResult<()> {
        let mut hb = self.handlebars.clone();
        hb.register_template_string("__validate__", template)
            .map_err(|e| TemplateError::InvalidTemplate(e.to_string()))?;
        Ok(())
    }

    /// Compile MJML to HTML (placeholder - would need mjml-rs or similar)
    pub fn compile_mjml(&self, mjml_source: &str) -> TemplateResult<String> {
        if !self.mjml_enabled {
            return Err(TemplateError::InvalidTemplate(
                "MJML compilation is not enabled".to_string(),
            ));
        }

        // In a full implementation, this would use mjml-rs or call an MJML service
        // For now, return a placeholder
        warn!("MJML compilation requested but not implemented");
        Ok(format!(
            "<!-- MJML would be compiled here -->\n{}",
            mjml_source
        ))
    }

    /// Extract variables from a template (basic implementation)
    pub fn extract_variables(&self, template: &str) -> Vec<String> {
        let mut variables = Vec::new();

        // Simple regex-like parsing for {{variable}} patterns
        // This is a basic implementation - Handlebars has more complex syntax
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second {
                let mut var_name = String::new();
                while let Some(c) = chars.next() {
                    if c == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second }
                        break;
                    }
                    var_name.push(c);
                }
                // Clean up the variable name (remove helpers, etc.)
                let clean_name = var_name
                    .split('|')
                    .next()
                    .unwrap_or(&var_name)
                    .trim()
                    .to_string();
                if !clean_name.is_empty()
                    && !clean_name.starts_with('#')
                    && !clean_name.starts_with('/')
                {
                    variables.push(clean_name);
                }
            }
        }

        variables.sort();
        variables.dedup();
        variables
    }

    /// Build sample payload from variable schema
    pub fn build_sample_payload(schema: &[TemplateVariable]) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        for var in schema {
            let value = if let Some(default) = &var.default_value {
                serde_json::Value::String(default.clone())
            } else {
                match var.name.as_str() {
                    "user_name" | "username" | "name" | "display_name" => {
                        serde_json::Value::String("John Doe".to_string())
                    }
                    "email" | "user_email" | "recipient_email" => {
                        serde_json::Value::String("user@example.com".to_string())
                    }
                    "site_name" | "app_name" => serde_json::Value::String("RustChat".to_string()),
                    "site_url" | "base_url" => {
                        serde_json::Value::String("https://chat.example.com".to_string())
                    }
                    "verification_link" | "reset_link" | "invite_link" | "action_link" => {
                        serde_json::Value::String(
                            "https://chat.example.com/action?token=abc123".to_string(),
                        )
                    }
                    "channel_name" => serde_json::Value::String("general".to_string()),
                    "team_name" => serde_json::Value::String("Engineering".to_string()),
                    "message_count" | "unread_count" => serde_json::Value::Number(5.into()),
                    "message_content" | "message_excerpt" => {
                        serde_json::Value::String("Hello! This is a sample message...".to_string())
                    }
                    "sender_name" => serde_json::Value::String("Jane Smith".to_string()),
                    "timestamp" | "date" | "created_at" => {
                        serde_json::Value::String("2026-02-22T20:00:00Z".to_string())
                    }
                    _ => serde_json::Value::String(format!("[{}]", var.name)),
                }
            };
            map.insert(var.name.clone(), value);
        }

        serde_json::Value::Object(map)
    }
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================
// Layout Wrapper Support
// ============================================

/// Email layout wrapper
#[derive(Debug, Clone)]
pub struct EmailLayout {
    pub name: String,
    pub html_template: String,
    pub text_template: Option<String>,
}

impl EmailLayout {
    /// Apply the layout to rendered content
    pub fn apply(&self, content: &RenderedEmail) -> RenderedEmail {
        // In a real implementation, we'd use Handlebars to inject
        // the content into the layout template
        // For now, simple string replacement
        let html = self.html_template.replace(
            "{{content}}",
            content.body_html.as_deref().unwrap_or(&content.body_text),
        );

        let text = self
            .text_template
            .as_ref()
            .map(|t| t.replace("{{content}}", &content.body_text))
            .unwrap_or_else(|| content.body_text.clone());

        RenderedEmail {
            subject: content.subject.clone(),
            body_text: text,
            body_html: Some(html),
        }
    }
}

// ============================================
// Localization Support
// ============================================

/// Fallback chain for locale resolution
pub struct LocaleFallback {
    pub user_locale: Option<String>,
    pub tenant_default: Option<String>,
    pub system_default: String,
}

impl LocaleFallback {
    pub fn resolve(&self) -> Vec<String> {
        let mut locales = Vec::new();

        if let Some(user) = &self.user_locale {
            locales.push(user.clone());
            // Add base locale (e.g., "en" from "en-US")
            if let Some(base) = user.split('-').next() {
                if base != user {
                    locales.push(base.to_string());
                }
            }
        }

        if let Some(tenant) = &self.tenant_default {
            if !locales.contains(tenant) {
                locales.push(tenant.clone());
            }
        }

        if !locales.contains(&self.system_default) {
            locales.push(self.system_default.clone());
        }

        locales
    }
}

// ============================================
// Variable Schema Helpers
// ============================================

/// Standard variable schemas for built-in workflows
pub struct StandardVariables;

impl StandardVariables {
    pub fn registration() -> Vec<TemplateVariable> {
        vec![
            TemplateVariable {
                name: "user_name".to_string(),
                required: true,
                default_value: None,
                description: Some("The user's display name".to_string()),
            },
            TemplateVariable {
                name: "email".to_string(),
                required: true,
                default_value: None,
                description: Some("The user's email address".to_string()),
            },
            TemplateVariable {
                name: "verification_link".to_string(),
                required: true,
                default_value: None,
                description: Some("Link to verify email address".to_string()),
            },
            TemplateVariable {
                name: "site_name".to_string(),
                required: false,
                default_value: Some("RustChat".to_string()),
                description: Some("The site name".to_string()),
            },
            TemplateVariable {
                name: "site_url".to_string(),
                required: false,
                default_value: None,
                description: Some("The site URL".to_string()),
            },
        ]
    }

    pub fn password_reset() -> Vec<TemplateVariable> {
        vec![
            TemplateVariable {
                name: "user_name".to_string(),
                required: true,
                default_value: None,
                description: Some("The user's display name".to_string()),
            },
            TemplateVariable {
                name: "reset_link".to_string(),
                required: true,
                default_value: None,
                description: Some("Link to reset password".to_string()),
            },
            TemplateVariable {
                name: "expiry_hours".to_string(),
                required: false,
                default_value: Some("24".to_string()),
                description: Some("Hours until link expires".to_string()),
            },
            TemplateVariable {
                name: "site_name".to_string(),
                required: false,
                default_value: Some("RustChat".to_string()),
                description: Some("The site name".to_string()),
            },
        ]
    }

    pub fn offline_messages() -> Vec<TemplateVariable> {
        vec![
            TemplateVariable {
                name: "user_name".to_string(),
                required: true,
                default_value: None,
                description: Some("The recipient's name".to_string()),
            },
            TemplateVariable {
                name: "channel_name".to_string(),
                required: true,
                default_value: None,
                description: Some("The channel name".to_string()),
            },
            TemplateVariable {
                name: "team_name".to_string(),
                required: false,
                default_value: None,
                description: Some("The team name".to_string()),
            },
            TemplateVariable {
                name: "message_count".to_string(),
                required: true,
                default_value: None,
                description: Some("Number of unread messages".to_string()),
            },
            TemplateVariable {
                name: "message_excerpt".to_string(),
                required: false,
                default_value: None,
                description: Some("Preview of the latest message".to_string()),
            },
            TemplateVariable {
                name: "sender_name".to_string(),
                required: false,
                default_value: None,
                description: Some("Name of the sender".to_string()),
            },
            TemplateVariable {
                name: "channel_link".to_string(),
                required: false,
                default_value: None,
                description: Some("Direct link to the channel".to_string()),
            },
        ]
    }

    pub fn announcement() -> Vec<TemplateVariable> {
        vec![
            TemplateVariable {
                name: "user_name".to_string(),
                required: false,
                default_value: None,
                description: Some("The recipient's name".to_string()),
            },
            TemplateVariable {
                name: "subject".to_string(),
                required: true,
                default_value: None,
                description: Some("The announcement subject".to_string()),
            },
            TemplateVariable {
                name: "content".to_string(),
                required: true,
                default_value: None,
                description: Some("The announcement content".to_string()),
            },
            TemplateVariable {
                name: "from_name".to_string(),
                required: false,
                default_value: None,
                description: Some("Name of the sender/admin".to_string()),
            },
            TemplateVariable {
                name: "unsubscribe_link".to_string(),
                required: false,
                default_value: None,
                description: Some("Link to unsubscribe from announcements".to_string()),
            },
        ]
    }
}

#[cfg(test)]
mod truncate_tests {
    use super::*;

    fn render_truncate(value: &str, length: i64) -> String {
        let renderer = TemplateRenderer::new();
        let context = RenderContext::new()
            .with_variables(serde_json::json!({ "value": value }))
            .expect("variables should serialize");
        let template = format!("{{{{truncate value {}}}}}", length);
        renderer
            .render_template(&template, &context)
            .expect("template should render")
    }

    #[test]
    fn truncate_ascii_is_unchanged() {
        assert_eq!(render_truncate("hello world", 5), "hello...");
        assert_eq!(render_truncate("short", 100), "short");
    }

    #[test]
    fn truncate_multibyte_boundary_does_not_panic() {
        // 4-byte emoji followed by CJK: byte 5 and 6 land inside multi-byte
        // characters. This must truncate at a character boundary, not panic.
        let text = "👍中文消息内容";
        let rendered = render_truncate(text, 5);
        assert!(rendered.ends_with("..."));
        assert!(!rendered.starts_with('\u{FFFD}'));
        // "👍" is 4 bytes; byte boundary 5 lands inside "中", so we expect
        // only the emoji to survive before the ellipsis.
        assert_eq!(rendered, "👍...");
    }

    #[test]
    fn truncate_cjk_only_input() {
        let rendered = render_truncate("中文消息", 3);
        // "中" is exactly 3 bytes, so it survives intact.
        assert_eq!(rendered, "中...");
        // Byte 4 lands inside the second CJK char: only the first survives.
        assert_eq!(render_truncate("中文消息", 4), "中...");
    }
}
