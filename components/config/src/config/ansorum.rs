use std::collections::HashSet;
use std::path::{Component, Path};

use errors::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use url::{Host, Url};

pub const DEFAULT_EVAL_MODEL: &str = "gpt-5.4-mini";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ansorum {
    pub redirects: Redirects,
    pub packs: Packs,
    pub eval: Eval,
    pub delivery: Delivery,
}

impl Ansorum {
    pub fn validate(&self, base_url: &Url) -> Result<()> {
        self.redirects.validate(base_url)?;
        self.packs.validate()?;
        self.eval.validate()?;
        self.delivery.validate()?;

        Ok(())
    }
}

impl Default for Ansorum {
    fn default() -> Self {
        Self {
            redirects: Redirects::default(),
            packs: Packs::default(),
            eval: Eval::default(),
            delivery: Delivery::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Redirects {
    pub external_host_allowlist: Vec<String>,
    pub routes: Vec<RedirectRoute>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedirectRoute {
    pub code: String,
    pub target: String,
}

impl Redirects {
    fn validate(&self, base_url: &Url) -> Result<()> {
        let mut seen = HashSet::new();
        for host in &self.external_host_allowlist {
            let host = host.trim();
            if host.is_empty() {
                bail!("ansorum.redirects.external_host_allowlist cannot contain empty hosts");
            }

            if host.contains("://") || host.contains('/') || host.contains('?') || host.contains('#')
            {
                bail!(
                    "Invalid ansorum.redirects.external_host_allowlist entry `{host}`: expected a bare host name"
                );
            }

            let normalized = host.to_ascii_lowercase();
            Host::parse(&normalized).map_err(|_| {
                anyhow!(
                    "Invalid ansorum.redirects.external_host_allowlist entry `{host}`: expected a valid host name"
                )
            })?;

            if !seen.insert(normalized) {
                bail!(
                    "Duplicate ansorum.redirects.external_host_allowlist entry `{host}` is not allowed"
                );
            }
        }

        let mut seen_codes = HashSet::new();
        for route in &self.routes {
            route.validate(base_url, &seen, &mut seen_codes)?;
        }

        Ok(())
    }
}

impl Default for Redirects {
    fn default() -> Self {
        Self { external_host_allowlist: Vec::new(), routes: Vec::new() }
    }
}

impl RedirectRoute {
    fn validate(
        &self,
        base_url: &Url,
        external_host_allowlist: &HashSet<String>,
        seen_codes: &mut HashSet<String>,
    ) -> Result<()> {
        let code = self.code.trim();
        if code.is_empty() {
            bail!("ansorum.redirects.routes.code cannot be empty");
        }

        if !code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            bail!(
                "Invalid ansorum.redirects.routes.code `{}`: use lowercase ASCII letters, digits, `-`, or `_`",
                self.code
            );
        }

        if !seen_codes.insert(code.to_string()) {
            bail!("Duplicate ansorum.redirects.routes.code `{code}` is not allowed");
        }

        validate_redirect_target(self.target.trim(), base_url, external_host_allowlist)?;

        Ok(())
    }
}

fn validate_redirect_target(
    target: &str,
    base_url: &Url,
    external_host_allowlist: &HashSet<String>,
) -> Result<()> {
    if target.is_empty() {
        bail!("ansorum.redirects.routes.target cannot be empty");
    }

    if target.starts_with("//") {
        bail!(
            "Invalid ansorum.redirects.routes.target `{target}`: protocol-relative URLs are not allowed"
        );
    }

    if target.starts_with('/') {
        return Ok(());
    }

    let url = Url::parse(target).map_err(|_| {
        anyhow!(
            "Invalid ansorum.redirects.routes.target `{target}`: expected an absolute http(s) URL or a site-relative path starting with `/`"
        )
    })?;

    if url.scheme() != "http" && url.scheme() != "https" {
        bail!(
            "Invalid ansorum.redirects.routes.target `{target}`: only http and https URLs are supported"
        );
    }

    if same_origin(base_url, &url) {
        return Ok(());
    }

    let host = url.host_str().ok_or_else(|| {
        anyhow!("Invalid ansorum.redirects.routes.target `{target}`: expected a host name")
    })?;
    let normalized_host = host.to_ascii_lowercase();
    if !external_host_allowlist.contains(&normalized_host) {
        bail!(
            "Invalid ansorum.redirects.routes.target `{target}`: external host `{host}` is not present in ansorum.redirects.external_host_allowlist"
        );
    }

    Ok(())
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Packs {
    pub auto_entity_packs: bool,
    pub auto_audience_packs: bool,
    pub curated: Vec<CuratedPack>,
}

impl Packs {
    fn validate(&self) -> Result<()> {
        let mut names = HashSet::new();
        let mut sources = HashSet::new();

        for pack in &self.curated {
            pack.validate()?;

            if !names.insert(pack.name.as_str()) {
                bail!("Duplicate ansorum.packs.curated name `{}` is not allowed", pack.name);
            }

            if !sources.insert(pack.source.as_str()) {
                bail!("Duplicate ansorum.packs.curated source `{}` is not allowed", pack.source);
            }
        }

        Ok(())
    }
}

impl Default for Packs {
    fn default() -> Self {
        Self { auto_entity_packs: true, auto_audience_packs: true, curated: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedPack {
    pub name: String,
    pub source: String,
}

impl CuratedPack {
    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("ansorum.packs.curated.name cannot be empty");
        }

        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            bail!(
                "Invalid ansorum.packs.curated name `{}`: use lowercase ASCII letters, digits, `-`, or `_`",
                self.name
            );
        }

        let source = self.source.trim();
        if source.is_empty() {
            bail!("ansorum.packs.curated.source cannot be empty");
        }

        let path = Path::new(source);
        if path.is_absolute() {
            bail!(
                "Invalid ansorum.packs.curated source `{source}`: absolute paths are not allowed"
            );
        }

        if path.components().any(|component| matches!(component, Component::ParentDir)) {
            bail!(
                "Invalid ansorum.packs.curated source `{source}`: parent directory traversal is not allowed"
            );
        }

        if !source.ends_with(".toml") {
            bail!("Invalid ansorum.packs.curated source `{source}`: expected a `.toml` file");
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalBackend {
    OpenAiResponses,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Eval {
    pub enabled: bool,
    pub backend: EvalBackend,
    pub model: Option<String>,
    pub api_base: Option<String>,
    pub prompt_version: String,
}

impl Eval {
    fn validate(&self) -> Result<()> {
        if let Some(model) = &self.model {
            if !model.starts_with("gpt-5.4") {
                bail!(
                    "Invalid ansorum.eval.model `{model}`: Ansorum eval currently supports GPT-5.4 family models only"
                );
            }
        }

        if let Some(api_base) = &self.api_base {
            let url = Url::parse(api_base).map_err(|_| {
                anyhow!("Invalid ansorum.eval.api_base `{api_base}`: expected an absolute URL")
            })?;

            if url.scheme() != "http" && url.scheme() != "https" {
                bail!(
                    "Invalid ansorum.eval.api_base `{api_base}`: only http and https URLs are supported"
                );
            }
        }

        if self.prompt_version.trim().is_empty() {
            bail!("ansorum.eval.prompt_version cannot be empty");
        }

        Ok(())
    }
}

impl Default for Eval {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: EvalBackend::OpenAiResponses,
            model: Some(DEFAULT_EVAL_MODEL.to_string()),
            api_base: None,
            prompt_version: "v1".to_string(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiVisibilityDefault {
    Public,
    SummaryOnly,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Delivery {
    pub markdown_routes: bool,
    pub markdown_negotiation: bool,
    pub default_ai_visibility: AiVisibilityDefault,
}

impl Delivery {
    fn validate(&self) -> Result<()> {
        if self.markdown_negotiation && !self.markdown_routes {
            bail!(
                "Invalid ansorum.delivery configuration: markdown_negotiation requires markdown_routes to be enabled"
            );
        }

        Ok(())
    }
}

impl Default for Delivery {
    fn default() -> Self {
        Self {
            markdown_routes: true,
            markdown_negotiation: true,
            default_ai_visibility: AiVisibilityDefault::Public,
        }
    }
}
