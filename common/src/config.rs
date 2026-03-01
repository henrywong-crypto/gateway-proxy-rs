use serde::Deserialize;

fn default_webfetch_agent_model() -> String {
    "us.anthropic.claude-haiku-4-5-20251001-v1:0".to_string()
}

fn default_webfetch_mock_prompt() -> String {
    "[Proxy mock] Web fetch intercepted. URL: '{{url}}'. No real fetch was performed.".to_string()
}

fn default_webfetch_redirect_prompt() -> String {
    "\
REDIRECT DETECTED: The URL redirects to a different host.

Original URL: {{original_url}}
Redirect URL: {{redirect_url}}
Status: {{status}}

To complete your request, I need to fetch content from the redirected URL. Please use WebFetch again with these parameters:
- url: \"{{redirect_url}}\"
- prompt: \"{{prompt}}\""
        .to_string()
}

fn default_webfetch_accept_prompt() -> String {
    "\
Web page content:
---
{{content}}
---

{{prompt}}

{{#if concise}}\
Provide a concise response based on the content above. Include relevant details, code examples, and documentation excerpts as needed.\
{{else}}\
Provide a concise response based only on the content above. In your response:
 - Enforce a strict 125-character maximum for quotes from any source document. Open Source Software is ok as long as we respect the license.
 - Use quotation marks for exact language from articles; any language outside of the quotation should never be word-for-word the same.
 - You are not a lawyer and never comment on the legality of your own prompts and responses.
 - Never produce or reproduce exact song lyrics.\
{{/if}}"
        .to_string()
}

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_webfetch_agent_model")]
    pub webfetch_agent_model: String,
    #[serde(default = "default_webfetch_mock_prompt")]
    pub webfetch_mock_prompt: String,
    #[serde(default = "default_webfetch_redirect_prompt")]
    pub webfetch_redirect_prompt: String,
    #[serde(default = "default_webfetch_accept_prompt")]
    pub webfetch_accept_prompt: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            webfetch_agent_model: default_webfetch_agent_model(),
            webfetch_mock_prompt: default_webfetch_mock_prompt(),
            webfetch_redirect_prompt: default_webfetch_redirect_prompt(),
            webfetch_accept_prompt: default_webfetch_accept_prompt(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
}
