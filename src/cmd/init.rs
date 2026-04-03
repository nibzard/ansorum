use std::fs::{canonicalize, create_dir};
use std::path::{Path, PathBuf};

use errors::{Result, bail};
use utils::fs::{create_directory, create_file};

#[cfg(test)]
use utils::fs::read_file;

use crate::cli::InitStarter;
use crate::prompt::ask_url;

const CONFIG: &str = r#"
base_url = "%BASE_URL%"
title = "%PROJECT_TITLE%"
description = "%PROJECT_DESCRIPTION%"
generate_feeds = false
generate_sitemap = true
generate_robots_txt = true
build_search_index = true
minify_html = true

[ansorum.redirects]
external_host_allowlist = ["docs.example.com"]

[[ansorum.redirects.routes]]
code = "sales-demo"
target = "https://docs.example.com/demo"

[[ansorum.redirects.routes]]
code = "billing-portal"
target = "/cancel/"

[ansorum.packs]
auto_entity_packs = false
auto_audience_packs = true

[[ansorum.packs.curated]]
name = "%CURATED_PACK_NAME%"
source = "%CURATED_PACK_SOURCE%"

[ansorum.eval]
enabled = false
model = "gpt-5.4-mini"
prompt_version = "%PROMPT_VERSION%"
"#;

const README: &str = r#"# %PROJECT_TITLE%

This project was scaffolded by `ansorum init`.

It includes:

- answer-first starter content in `content/`
- starter templates in `templates/`
- a built-in stylesheet at `static/site.css`
- a JSON-LD sidecar example at `content/refunds.schema.json`
- a curated pack definition in `collections/packs/billing.toml`
- deterministic eval fixtures in `eval/fixtures.yaml`

Run the full workflow:

```bash
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

Use `ansorum eval --llm` only when `OPENAI_API_KEY` is set and you want OpenAI
Responses API rubric scoring.
"#;

const HOME: &str = r#"+++
title = "%PROJECT_TITLE%"
description = "Answer-first documentation optimized for human readers, search engines, and AI systems."
+++

%PROJECT_TITLE% ships with an answer-center starter designed for clear human
navigation and clean machine-readable outputs from the same source.

Start with the public answers below, then adapt the content, templates, and
styling to your own domain and support surface.
"#;

const REFUNDS: &str = r#"+++
title = "Refund policy"
description = "How refunds work, who qualifies, and when payment returns land."
weight = 10

id = "refunds-policy"
summary = "How refunds work, who qualifies, and when payment returns land."
canonical_questions = ["how do refunds work", "can i get a refund"]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["cancel-subscription"]
external_refs = ["https://example.com/refunds"]
schema_type = "FAQPage"
review_by = 2026-06-01
owner = "Billing"
confidence_notes = "Reviewed by policy and support"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["refund policy", "refund rules"]

[extra]
homepage = true
+++

Refund details for customers.

## Eligibility

Refunds follow the [billing policy](https://example.com/policy).
"#;

const CANCEL: &str = r#"+++
title = "Cancel a subscription"
description = "How to cancel a subscription and what happens after."
weight = 20

id = "cancel-subscription"
summary = "How to cancel a subscription and what happens after."
canonical_questions = ["how do i cancel my subscription"]
intent = "task"
entity = "billing"
audience = "customer"
related = ["refunds-policy"]
external_refs = []
schema_type = "HowTo"
review_by = 2026-06-01
owner = "Billing Operations"
confidence_notes = "Matches current customer portal workflow"
visibility = "public"
ai_visibility = "summary_only"
llms_priority = "optional"
token_budget = "small"
retrieval_aliases = ["cancel subscription"]

[extra]
homepage = true
+++

Cancellation details for customers.

## Keep access

Use the [billing portal](https://example.com/billing) to manage changes.
"#;

const INTERNAL_PLAYBOOK: &str = r#"+++
title = "Internal support escalation"
description = "Internal escalation process for complex billing cases."
weight = 90

id = "internal-support-escalation"
summary = "Internal escalation process for complex billing cases."
canonical_questions = ["how do support agents escalate billing issues"]
intent = "reference"
entity = "support"
audience = "internal"
related = []
external_refs = []
schema_type = "Article"
visibility = "internal"
ai_visibility = "hidden"
llms_priority = "hidden"
token_budget = "small"
retrieval_aliases = ["billing escalation playbook"]
+++

Escalation details for internal teams only.
"#;

const REFUNDS_SCHEMA: &str = r#"{
  "publisher": {
    "@type": "Organization",
    "name": "Ansorum Billing"
  },
  "mainEntity": [
    {
      "@type": "Question",
      "name": "Who qualifies for a refund?",
      "acceptedAnswer": {
        "@type": "Answer",
        "text": "Most unused subscriptions are refundable within 30 days."
      }
    }
  ]
}
"#;

const BILLING_PACK: &str = r#"title = "Billing answers"
description = "Curated billing pack for customer-visible billing answers."
answers = ["refunds-policy", "cancel-subscription"]
"#;

const EVAL_FIXTURES: &str = r#"- question: can i get a refund after 30 days
  expected_ids: [refunds-policy]
  forbidden_ids: [internal-support-escalation]
  required_terms: [eligibility, billing policy]
  rubric_focus: reflect refund eligibility policy without using internal-only content

- question: how do i cancel my subscription
  expected_ids: [cancel-subscription]
  forbidden_ids: [internal-support-escalation]
  required_terms: [canonical page, cancel]
  rubric_focus: prefer the public cancellation answer and keep canonical links visible
"#;

const README_AI_REFERENCE_LAYER: &str = r#"# %PROJECT_TITLE%

This project was scaffolded by `ansorum init --starter ai-reference-layer`.

It includes an opinionated AI reference layer:

- definitions in `content/definitions/`
- playbooks in `content/playbooks/`
- methodology pages in `content/methodology/`
- metrics specs in `content/metrics/`
- comparison pages in `content/comparisons/`
- case studies in `content/case-studies/`
- a curated machine pack in `collections/packs/reference-layer.toml`

Run the full workflow:

```bash
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

Use `ansorum build --format json-stream` or `ansorum audit --format json-stream` when the CLI is being driven by an AI agent.
"#;

const HOME_AI_REFERENCE_LAYER: &str = r#"+++
title = "%PROJECT_TITLE%"
description = "Maintained AI reference layer for definitions, methodology, metrics, comparisons, and case studies."
+++

%PROJECT_TITLE% is scaffolded as an AI reference layer. The same authored corpus
drives human pages, machine markdown, scoped packs, and answer indexes.

Start with the archetypes below, then replace the placeholder content with your
canonical definitions, operating playbooks, methodology pages, and proof points.
"#;

const AI_DEFINITION: &str = r#"+++
title = "What is AI brand alignment?"
description = "Definition of AI brand alignment and what it controls."
weight = 10

id = "ai-brand-alignment"
summary = "Definition of AI brand alignment and what it controls."
canonical_questions = ["what is ai brand alignment", "define ai brand alignment"]
intent = "concept"
entity = "brand"
audience = "customer"
related = ["reference-layer-methodology", "citation-coverage", "ai-reference-layer-vs-docs-portal"]
external_refs = []
schema_type = "DefinedTerm"
review_by = 2026-06-01
priority = "high"
owner = "Brand Systems"
confidence_notes = "Canonical definition reviewed by product marketing"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["brand alignment ai", "ai alignment definition"]

[extra]
homepage = true
+++

AI brand alignment is the discipline of making the answers an AI system sees
match the product's canonical definitions, positioning, and proof.
"#;

const AI_PLAYBOOK: &str = r#"+++
title = "How to improve AI citations"
description = "Playbook for increasing citation quality from answer systems."
weight = 20

id = "improve-ai-citations"
summary = "Playbook for increasing citation quality from answer systems."
canonical_questions = ["how do we improve ai citations", "how to improve ai citations"]
intent = "task"
entity = "citations"
audience = "customer"
related = ["ai-brand-alignment", "citation-coverage", "reference-layer-methodology", "billing-reference-case-study"]
external_refs = []
schema_type = "HowTo"
review_by = 2026-06-01
priority = "high"
owner = "Growth Engineering"
confidence_notes = "Based on current publishing and eval workflow"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["citation playbook", "improve citations"]

[extra]
homepage = true
+++

Treat citations as an output of corpus quality, not a post-processing trick.

1. Publish one canonical answer per intent.
2. Keep related links between definitions, methods, metrics, and case studies.
3. Expose canonical markdown and answer indexes from the same source.
4. Audit freshness and rerun eval after every material change.
"#;

const AI_METHODOLOGY: &str = r#"+++
title = "AI reference layer methodology"
description = "Method for building and governing an AI reference layer."
weight = 30

id = "reference-layer-methodology"
summary = "Method for building and governing an AI reference layer."
canonical_questions = ["what is the ai reference layer methodology", "how do we build an ai reference layer"]
intent = "reference"
entity = "methodology"
audience = "customer"
related = ["ai-brand-alignment", "improve-ai-citations", "citation-coverage", "billing-reference-case-study"]
external_refs = []
schema_type = "Article"
review_by = 2026-06-01
priority = "high"
owner = "Documentation Platform"
confidence_notes = "Canonical operating method for corpus governance"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["reference layer method", "ai corpus methodology"]

[extra]
homepage = true
+++

The method is simple:

- define the canonical concepts
- connect them to operating playbooks
- specify the metrics used to validate output quality
- publish comparisons and case studies that bound claims with evidence
"#;

const AI_METRIC: &str = r#"+++
title = "Citation coverage"
description = "Metric definition for how often answers return usable citations."
weight = 40

id = "citation-coverage"
summary = "Metric definition for how often answers return usable citations."
canonical_questions = ["what is citation coverage", "how do we measure citation coverage"]
intent = "reference"
entity = "metrics"
audience = "customer"
related = ["improve-ai-citations", "reference-layer-methodology", "billing-reference-case-study"]
external_refs = []
schema_type = "Article"
review_by = 2026-06-01
owner = "Analytics"
confidence_notes = "Definition aligned with current eval rubric"
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "small"
retrieval_aliases = ["citation rate", "citation metric"]
+++

Citation coverage is the share of evaluated answers that include at least one
useful, inspectable citation to the canonical source.
"#;

const AI_COMPARISON: &str = r#"+++
title = "AI reference layer vs docs portal"
description = "Comparison between a maintained AI reference layer and a generic docs portal."
weight = 50

id = "ai-reference-layer-vs-docs-portal"
summary = "Comparison between a maintained AI reference layer and a generic docs portal."
canonical_questions = ["ai reference layer vs docs portal", "why not just use docs"]
intent = "comparison"
entity = "positioning"
audience = "customer"
related = ["ai-brand-alignment", "reference-layer-methodology", "billing-reference-case-study"]
external_refs = []
schema_type = "Article"
review_by = 2026-06-01
owner = "Product Marketing"
confidence_notes = "Comparison intended for product and GTM alignment"
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["docs portal comparison", "reference layer comparison"]
+++

An AI reference layer is opinionated and canonical. A docs portal may contain
good information, but it usually mixes navigation content, release notes, and
partial answers in ways that are harder for models to learn from reliably.
"#;

const AI_CASE_STUDY: &str = r#"+++
title = "Billing answer layer case study"
description = "Example case study showing how a governed answer layer improves billing answers."
weight = 60

id = "billing-reference-case-study"
summary = "Example case study showing how a governed answer layer improves billing answers."
canonical_questions = ["show me an ai reference layer case study", "billing answer layer case study"]
intent = "reference"
entity = "case-study"
audience = "customer"
related = ["reference-layer-methodology", "improve-ai-citations", "citation-coverage", "ai-reference-layer-vs-docs-portal"]
external_refs = []
schema_type = "Article"
review_by = 2026-06-01
owner = "Solutions"
confidence_notes = "Illustrative example starter meant to be replaced with real data"
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["reference layer case study", "billing case study"]
+++

Before the answer layer, billing answers were split across FAQs, docs, and
support snippets. After consolidation, the team published one maintained corpus
with explicit freshness metadata and measurable eval targets.
"#;

const AI_REFERENCE_PACK: &str = r#"title = "AI reference layer"
description = "Curated public pack for the AI reference layer starter."
answers = [
  "ai-brand-alignment",
  "improve-ai-citations",
  "reference-layer-methodology",
  "citation-coverage",
  "ai-reference-layer-vs-docs-portal",
  "billing-reference-case-study",
]
"#;

const AI_REFERENCE_EVAL_FIXTURES: &str = r#"- question: what is ai brand alignment
  expected_ids: [ai-brand-alignment]
  required_terms: [definition, canonical]
  rubric_focus: define the term clearly and keep the answer canonical

- question: how do we improve ai citations
  expected_ids: [improve-ai-citations]
  required_terms: [canonical answer, citations]
  rubric_focus: describe the publishing and governance steps that improve citations
"#;

const AI_SECTION_INDEX: &str = r#"+++
title = "%SECTION_TITLE%"
sort_by = "weight"
+++

%SECTION_DESCRIPTION%
"#;

const BASE_TEMPLATE: &str = r##"{% import "macros.html" as macros %}
<!DOCTYPE html>
<html lang="{{ config.default_language | default(value="en") }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    {% set current_title = config.title %}
    {% set current_description = config.description %}
    {% set current_url = config.base_url %}
    {% set og_type = "website" %}
    {% if section is defined %}
        {% set current_title = section.title | default(value=config.title) %}
        {% if section.description %}
            {% set current_description = section.description %}
        {% endif %}
        {% set current_url = section.permalink %}
    {% endif %}
    {% if page is defined %}
        {% set current_title = page.title %}
        {% if page.description %}
            {% set current_description = page.description %}
        {% endif %}
        {% set current_url = page.permalink %}
        {% set og_type = "article" %}
    {% endif %}
    <title>{% block title %}{% if current_title == config.title %}{{ config.title }}{% else %}{{ current_title }} | {{ config.title }}{% endif %}{% endblock %}</title>
    <meta name="description" content="{{ current_description }}">
    <meta name="robots" content="index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1">
    <link rel="canonical" href="{{ current_url | safe }}">
    <meta property="og:site_name" content="{{ config.title }}">
    <meta property="og:type" content="{{ og_type }}">
    <meta property="og:title" content="{{ current_title }}">
    <meta property="og:description" content="{{ current_description }}">
    <meta property="og:url" content="{{ current_url | safe }}">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="{{ current_title }}">
    <meta name="twitter:description" content="{{ current_description }}">
    <meta name="theme-color" content="#b45a3c">
    <link rel="alternate" type="text/plain" href="{{ get_url(path='llms.txt') | safe }}" title="llms.txt">
    <link rel="alternate" type="application/json" href="{{ get_url(path='answers.json') | safe }}" title="answers.json">
    {% if page is defined %}
    <link rel="alternate" type="text/markdown" href="{{ macros::machine_markdown_url(path=page.path) | safe }}" title="{{ page.title }} markdown">
    {% endif %}
    <link rel="stylesheet" href="{{ get_url(path='site.css') | safe }}">
    {% block extra_head %}{% endblock %}
</head>
<body class="{% block body_class %}starter{% endblock %}">
    {% set home = get_section(path='_index.md', required=false) %}
    <div class="shell">
        <header class="masthead">
            <div class="masthead__inner">
                <a class="brand" href="{{ config.base_url | safe }}">{{ config.title }}</a>
                <div class="brand__meta">Answer-first documentation for humans and machines.</div>
                <nav class="topnav" aria-label="Primary">
                    <a href="{{ config.base_url | safe }}">Home</a>
                    <a href="{{ get_url(path='llms.txt') | safe }}">llms.txt</a>
                    <a href="{{ get_url(path='answers.json') | safe }}">answers.json</a>
                </nav>
            </div>
        </header>

        <main class="main">
            {% block content %}{% endblock %}
        </main>

        <footer class="footer">
            <div class="footer__grid">
                <div>
                    <h2>Why this starter works</h2>
                    <p>Readable HTML, canonical Markdown, answer indexes, and JSON-LD sidecars come from one authored corpus.</p>
                </div>
                <div>
                    <h2>Machine surfaces</h2>
                    <ul>
                        <li><a href="{{ get_url(path='llms.txt') | safe }}">llms.txt</a></li>
                        <li><a href="{{ get_url(path='answers.json') | safe }}">answers.json</a></li>
                        <li><a href="{{ get_url(path='sitemap.xml') | safe }}">sitemap.xml</a></li>
                    </ul>
                </div>
            </div>
        </footer>
    </div>
</body>
</html>
"##;

const MACROS_TEMPLATE: &str = r#"{% macro machine_markdown_url(path) %}
{% set normalized = path | trim_start_matches(pat="/") | trim_end_matches(pat="/") %}
{% if normalized == "" %}{{ get_url(path="index.md") | safe }}{% else %}{{ get_url(path=normalized ~ ".md") | safe }}{% endif %}
{% endmacro machine_markdown_url %}
"#;

const INDEX_TEMPLATE: &str = r#"{% import "macros.html" as macros %}
{% extends "base.html" %}

{% block content %}
<section class="hero">
    <p class="eyebrow">SEO, AEO, and GEO ready</p>
    <h1>{{ section.title | default(value=config.title) }}</h1>
    <p class="lede">{{ section.description | default(value=config.description) }}</p>
    <div class="hero__body">
        {{ section.content | safe }}
    </div>
    <div class="hero__actions">
        <a class="button button--primary" href="{{ get_url(path='llms.txt') | safe }}">Open llms.txt</a>
        <a class="button" href="{{ get_url(path='answers.json') | safe }}">Browse answers.json</a>
    </div>
</section>

<section class="answers" aria-labelledby="featured-answers">
    <div class="section-heading">
        <p class="eyebrow">Public answer corpus</p>
        <h2 id="featured-answers">Start with these authoritative answers</h2>
    </div>
    <div class="answer-grid">
        {% for entry in section.pages %}
            {% if entry.extra.homepage | default(value=false) %}
            <article class="answer-card">
                <p class="answer-card__label">Canonical answer</p>
                <h3><a href="{{ entry.permalink | safe }}">{{ entry.title }}</a></h3>
                {% if entry.description %}
                <p>{{ entry.description }}</p>
                {% endif %}
                <div class="answer-card__links">
                    <a href="{{ entry.permalink | safe }}">Read page</a>
                    <a href="{{ macros::machine_markdown_url(path=entry.path) | safe }}">Markdown</a>
                </div>
            </article>
            {% endif %}
        {% endfor %}
    </div>
</section>
{% endblock %}
"#;

const PAGE_TEMPLATE: &str = r#"{% import "macros.html" as macros %}
{% extends "base.html" %}

{% block body_class %}starter starter--page{% endblock %}

{% block content %}
<article class="article">
    <header class="article__header">
        <p class="eyebrow">Canonical answer</p>
        <h1>{{ page.title }}</h1>
        {% if page.description %}
        <p class="article__summary">{{ page.description }}</p>
        {% endif %}
        <div class="article__actions">
            <a class="button button--primary" href="{{ macros::machine_markdown_url(path=page.path) | safe }}">Open Markdown</a>
            <a class="button" href="{{ get_url(path='llms.txt') | safe }}">View llms.txt</a>
        </div>
    </header>

    <div class="article__content">
        {{ page.content | safe }}
    </div>

    <aside class="article__meta">
        <h2>Machine-friendly surfaces</h2>
        <ul>
            <li><a href="{{ macros::machine_markdown_url(path=page.path) | safe }}">Canonical Markdown</a></li>
            <li><a href="{{ get_url(path='answers.json') | safe }}">Answer index</a></li>
            <li><a href="{{ get_url(path='llms.txt') | safe }}">llms.txt</a></li>
        </ul>
        {% if page.answer %}
        <h2>Governance</h2>
        <dl class="article__governance">
            {% if page.answer.review_by %}
            <div>
                <dt>Review by</dt>
                <dd>{{ page.answer.review_by }}</dd>
            </div>
            {% endif %}
            {% if page.answer.owner %}
            <div>
                <dt>Owner</dt>
                <dd>{{ page.answer.owner }}</dd>
            </div>
            {% endif %}
            {% if page.answer.confidence_notes %}
            <div>
                <dt>Confidence</dt>
                <dd>{{ page.answer.confidence_notes }}</dd>
            </div>
            {% endif %}
        </dl>
        {% endif %}
    </aside>
</article>
{% endblock %}
"#;

const SITE_CSS: &str = r#":root {
  --paper: #f7f1e8;
  --paper-strong: #fffaf2;
  --ink: #1f1a17;
  --ink-muted: #5d5148;
  --accent: #b45a3c;
  --accent-strong: #8f3e21;
  --line: rgba(31, 26, 23, 0.12);
  --shadow: 0 24px 60px rgba(83, 49, 31, 0.14);
  --radius: 22px;
  --content: 74ch;
}

* { box-sizing: border-box; }

html {
  background:
    radial-gradient(circle at top, rgba(180, 90, 60, 0.14), transparent 34%),
    linear-gradient(180deg, #fbf5ec 0%, var(--paper) 60%, #f1e6d9 100%);
  color: var(--ink);
  font-family: Charter, "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, "Noto Serif", serif;
  line-height: 1.65;
}

body { margin: 0; }

a {
  color: var(--accent-strong);
  text-decoration-thickness: 0.08em;
  text-underline-offset: 0.16em;
}

img { max-width: 100%; }

.shell {
  min-height: 100vh;
  padding: 0 1.25rem 4rem;
}

.masthead__inner,
.main,
.footer {
  max-width: 1120px;
  margin: 0 auto;
}

.masthead {
  padding: 1.25rem 0 0;
}

.masthead__inner {
  display: grid;
  gap: 0.5rem;
  padding: 1.25rem 1.5rem;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: rgba(255, 250, 242, 0.8);
  backdrop-filter: blur(12px);
  box-shadow: var(--shadow);
}

.brand {
  font-size: 1.3rem;
  font-weight: 700;
  color: var(--ink);
  text-decoration: none;
}

.brand__meta,
.eyebrow,
.footer p,
.footer li,
.answer-card__label {
  color: var(--ink-muted);
}

.topnav {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}

.topnav a {
  font-size: 0.96rem;
  text-decoration: none;
}

.main {
  padding-top: 2rem;
}

.hero,
.article {
  padding: clamp(1.5rem, 3vw, 3rem);
  border: 1px solid var(--line);
  border-radius: calc(var(--radius) + 4px);
  background: linear-gradient(180deg, rgba(255, 250, 242, 0.96), rgba(255, 248, 238, 0.88));
  box-shadow: var(--shadow);
}

.hero {
  position: relative;
  overflow: hidden;
}

.hero::after {
  content: "";
  position: absolute;
  inset: auto -8% -18% auto;
  width: 18rem;
  height: 18rem;
  border-radius: 999px;
  background: radial-gradient(circle, rgba(180, 90, 60, 0.2), transparent 70%);
}

.hero h1,
.article h1,
.section-heading h2 {
  margin: 0;
  line-height: 1.08;
  letter-spacing: -0.03em;
}

.hero h1 { font-size: clamp(2.4rem, 6vw, 4.8rem); max-width: 11ch; }
.section-heading h2 { font-size: clamp(1.6rem, 3vw, 2.5rem); }

.lede,
.article__summary {
  max-width: 48rem;
  font-size: 1.12rem;
  color: var(--ink-muted);
}

.hero__body,
.article__content {
  max-width: var(--content);
}

.hero__actions,
.article__actions,
.answer-card__links {
  display: flex;
  flex-wrap: wrap;
  gap: 0.85rem;
  margin-top: 1.25rem;
}

.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.8rem 1.1rem;
  border-radius: 999px;
  border: 1px solid rgba(180, 90, 60, 0.28);
  background: rgba(255, 250, 242, 0.9);
  color: var(--ink);
  text-decoration: none;
}

.button--primary {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.answers {
  margin-top: 2rem;
}

.section-heading {
  margin-bottom: 1rem;
}

.answer-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
}

.answer-card {
  padding: 1.25rem;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: rgba(255, 250, 242, 0.84);
  box-shadow: 0 16px 40px rgba(83, 49, 31, 0.08);
}

.answer-card h3 {
  margin: 0.15rem 0 0.5rem;
  font-size: 1.28rem;
}

.article {
  display: grid;
  gap: 2rem;
}

.article__meta {
  padding-top: 1rem;
  border-top: 1px solid var(--line);
}

.article__governance {
  display: grid;
  gap: 0.85rem;
}

.article__governance dt {
  font-size: 0.88rem;
  font-weight: 700;
  color: var(--ink-muted);
}

.article__governance dd {
  margin: 0.15rem 0 0;
}

.article__content :where(h2, h3, h4) {
  margin-top: 2rem;
  line-height: 1.15;
}

.article__content code {
  padding: 0.15rem 0.35rem;
  border-radius: 0.4rem;
  background: rgba(31, 26, 23, 0.07);
}

.article__content pre {
  overflow-x: auto;
  padding: 1rem;
  border-radius: 16px;
  background: #231c19;
  color: #f9efe0;
}

.footer {
  margin-top: 2rem;
  padding: 1.5rem;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: rgba(255, 250, 242, 0.82);
}

.footer__grid {
  display: grid;
  gap: 1.5rem;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
}

@media (max-width: 720px) {
  .shell { padding-inline: 0.85rem; }
  .masthead__inner,
  .hero,
  .article,
  .footer { padding: 1.1rem; }
  .hero h1 { max-width: 100%; }
}
"#;

// canonicalize(path) function on windows system returns a path with UNC.
// Example: \\?\C:\Users\VssAdministrator\AppData\Local\Temp\new_project
// More details on Universal Naming Convention (UNC):
// https://en.wikipedia.org/wiki/Path_(computing)#Uniform_Naming_Convention
// So the following const will be used to remove the network part of the UNC to display users a more common
// path on windows systems.
// This is a workaround until this issue https://github.com/rust-lang/rust/issues/42869 was fixed.
const LOCAL_UNC: &str = "\\\\?\\";

// Given a path, return true if it is a directory and it doesn't have any
// non-hidden files, otherwise return false (path is assumed to exist)
pub fn is_directory_quasi_empty(path: &Path) -> Result<bool> {
    if path.is_dir() {
        let mut entries = match path.read_dir() {
            Ok(entries) => entries,
            Err(e) => {
                bail!(
                    "Could not read `{}` because of error: {}",
                    path.to_string_lossy().to_string(),
                    e
                );
            }
        };
        // If any entry raises an error or isn't hidden (i.e. starts with `.`), we raise an error
        if entries.any(|x| match x {
            Ok(file) => !file
                .file_name()
                .to_str()
                .expect("Could not convert filename to &str")
                .starts_with('.'),
            Err(_) => true,
        }) {
            return Ok(false);
        }
        return Ok(true);
    }

    Ok(false)
}

// Remove the unc part of a windows path
fn strip_unc(path: &Path) -> String {
    let path_to_refine = path.to_str().unwrap();
    path_to_refine.trim_start_matches(LOCAL_UNC).to_string()
}

fn starter_project_description(starter: InitStarter) -> &'static str {
    match starter {
        InitStarter::AnswerFirst => "Starter Ansorum project for an answer-first knowledge corpus.",
        InitStarter::AiReferenceLayer => {
            "Starter Ansorum project for a maintained AI reference layer."
        }
    }
}

fn starter_pack_name(starter: InitStarter) -> &'static str {
    match starter {
        InitStarter::AnswerFirst => "billing",
        InitStarter::AiReferenceLayer => "reference-layer",
    }
}

fn starter_pack_source(starter: InitStarter) -> &'static str {
    match starter {
        InitStarter::AnswerFirst => "collections/packs/billing.toml",
        InitStarter::AiReferenceLayer => "collections/packs/reference-layer.toml",
    }
}

fn starter_prompt_version(starter: InitStarter) -> &'static str {
    match starter {
        InitStarter::AnswerFirst => "starter-v1",
        InitStarter::AiReferenceLayer => "ai-reference-layer-v1",
    }
}

pub fn create_new_project(name: &str, force: bool, starter: InitStarter) -> Result<()> {
    let path = Path::new(name);

    // Better error message than the rust default
    if path.exists() && !is_directory_quasi_empty(path)? && !force {
        if name == "." {
            bail!("The current directory is not an empty folder (hidden files are ignored).");
        } else {
            bail!(
                "`{}` is not an empty folder (hidden files are ignored).",
                path.to_string_lossy().to_string()
            )
        }
    }

    console::info("Welcome to Ansorum!");
    match starter {
        InitStarter::AnswerFirst => console::info(
            "This scaffold creates an answer-first project with starter content, packs, redirects, and eval fixtures.",
        ),
        InitStarter::AiReferenceLayer => console::info(
            "This scaffold creates an AI reference layer with definitions, playbooks, methodology pages, metrics, comparisons, and case studies.",
        ),
    }
    console::info("Any choices made can be changed by modifying the generated files later.");

    let base_url = ask_url("> What is the URL of your site?", "https://example.com")?;
    let project_title = project_title(path);

    let config = CONFIG
        .trim_start()
        .replace("%BASE_URL%", &base_url)
        .replace("%PROJECT_TITLE%", &project_title)
        .replace("%PROJECT_DESCRIPTION%", starter_project_description(starter))
        .replace("%CURATED_PACK_NAME%", starter_pack_name(starter))
        .replace("%CURATED_PACK_SOURCE%", starter_pack_source(starter))
        .replace("%PROMPT_VERSION%", starter_prompt_version(starter));

    populate(path, &project_title, &config, starter)?;

    println!();
    console::success(&format!(
        "Done! Your {} project was created in {}",
        match starter {
            InitStarter::AnswerFirst => "answer-first",
            InitStarter::AiReferenceLayer => "AI reference layer",
        },
        strip_unc(&canonicalize(path).unwrap())
    ));
    println!();
    console::info(
        "Next steps: `ansorum build`, `ansorum serve`, `ansorum audit`, and `ansorum eval`.",
    );
    println!("Visit https://ansorum.com/documentation/ for the full documentation.");
    Ok(())
}

fn populate(path: &Path, project_title: &str, config: &str, starter: InitStarter) -> Result<()> {
    if !path.exists() {
        create_dir(path)?;
    }

    create_file(&path.join("config.toml"), config)?;
    let readme = match starter {
        InitStarter::AnswerFirst => README.replace("%PROJECT_TITLE%", project_title),
        InitStarter::AiReferenceLayer => {
            README_AI_REFERENCE_LAYER.replace("%PROJECT_TITLE%", project_title)
        }
    };
    create_file(&path.join("README.md"), readme)?;

    create_directory(&path.join("collections/packs"))?;
    create_directory(&path.join("content"))?;
    create_directory(&path.join("eval"))?;
    create_directory(&path.join("templates"))?;
    create_directory(&path.join("static"))?;
    create_file(&path.join("templates/base.html"), BASE_TEMPLATE)?;
    create_file(&path.join("templates/macros.html"), MACROS_TEMPLATE)?;
    create_file(&path.join("templates/index.html"), INDEX_TEMPLATE)?;
    create_file(&path.join("templates/page.html"), PAGE_TEMPLATE)?;
    create_file(&path.join("static/site.css"), SITE_CSS)?;

    match starter {
        InitStarter::AnswerFirst => {
            create_file(&path.join("collections/packs/billing.toml"), BILLING_PACK)?;
            create_file(
                &path.join("content/_index.md"),
                HOME.replace("%PROJECT_TITLE%", project_title),
            )?;
            create_file(&path.join("content/refunds.md"), REFUNDS)?;
            create_file(&path.join("content/cancel.md"), CANCEL)?;
            create_file(&path.join("content/internal-playbook.md"), INTERNAL_PLAYBOOK)?;
            create_file(&path.join("content/refunds.schema.json"), REFUNDS_SCHEMA)?;
            create_file(&path.join("eval/fixtures.yaml"), EVAL_FIXTURES)?;
        }
        InitStarter::AiReferenceLayer => {
            create_directory(&path.join("content/definitions"))?;
            create_directory(&path.join("content/playbooks"))?;
            create_directory(&path.join("content/methodology"))?;
            create_directory(&path.join("content/metrics"))?;
            create_directory(&path.join("content/comparisons"))?;
            create_directory(&path.join("content/case-studies"))?;
            create_file(
                &path.join("content/definitions/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Definitions")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "Canonical definitions that anchor the answer graph.",
                    ),
            )?;
            create_file(
                &path.join("content/playbooks/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Playbooks")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "Operational answer patterns teams can follow directly.",
                    ),
            )?;
            create_file(
                &path.join("content/methodology/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Methodology")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "The governing method behind the AI reference layer.",
                    ),
            )?;
            create_file(
                &path.join("content/metrics/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Metrics")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "Metric definitions used to validate answer quality.",
                    ),
            )?;
            create_file(
                &path.join("content/comparisons/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Comparisons")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "Comparison pages that clarify boundaries and tradeoffs.",
                    ),
            )?;
            create_file(
                &path.join("content/case-studies/_index.md"),
                AI_SECTION_INDEX
                    .replace("%SECTION_TITLE%", "Case Studies")
                    .replace(
                        "%SECTION_DESCRIPTION%",
                        "Proof pages that show the system working in practice.",
                    ),
            )?;
            create_file(
                &path.join("collections/packs/reference-layer.toml"),
                AI_REFERENCE_PACK,
            )?;
            create_file(
                &path.join("content/_index.md"),
                HOME_AI_REFERENCE_LAYER.replace("%PROJECT_TITLE%", project_title),
            )?;
            create_file(
                &path.join("content/definitions/ai-brand-alignment.md"),
                AI_DEFINITION,
            )?;
            create_file(
                &path.join("content/playbooks/improve-ai-citations.md"),
                AI_PLAYBOOK,
            )?;
            create_file(
                &path.join("content/methodology/reference-layer-methodology.md"),
                AI_METHODOLOGY,
            )?;
            create_file(
                &path.join("content/metrics/citation-coverage.md"),
                AI_METRIC,
            )?;
            create_file(
                &path.join("content/comparisons/ai-reference-layer-vs-docs-portal.md"),
                AI_COMPARISON,
            )?;
            create_file(
                &path.join("content/case-studies/billing-reference-case-study.md"),
                AI_CASE_STUDY,
            )?;
            create_file(&path.join("eval/fixtures.yaml"), AI_REFERENCE_EVAL_FIXTURES)?;
        }
    }

    Ok(())
}

fn project_title(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != ".")
        .map(ToOwned::to_owned)
        .or_else(|| {
            canonicalize(path)
                .ok()
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .or_else(|| current_dir_name())
        .unwrap_or_else(|| "ansorum-answers".to_string());

    title_case_slug(&file_name)
}

fn current_dir_name() -> Option<String> {
    canonicalize(PathBuf::from("."))
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn title_case_slug(input: &str) -> String {
    let words: Vec<String> = input
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str().to_ascii_lowercase())
                }
                None => String::new(),
            }
        })
        .filter(|part| !part.is_empty())
        .collect();

    if words.is_empty() { "Ansorum Answers".to_string() } else { words.join(" ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs::{create_dir, remove_dir, remove_dir_all};
    use std::path::Path;

    #[test]
    fn init_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&dir).unwrap();
        assert!(allowed);
    }

    #[test]
    fn init_non_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_non_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let mut content = dir.clone();
        content.push("content");
        create_dir(&content).unwrap();
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&content).unwrap();
        remove_dir(&dir).unwrap();
        assert!(!allowed);
    }

    #[test]
    fn init_quasi_empty_directory() {
        let mut dir = temp_dir();
        dir.push("test_quasi_empty_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let mut git = dir.clone();
        git.push(".git");
        create_dir(&git).unwrap();
        let allowed = is_directory_quasi_empty(&dir)
            .expect("An error happened reading the directory's contents");
        remove_dir(&git).unwrap();
        remove_dir(&dir).unwrap();
        assert!(allowed);
    }

    #[test]
    fn populate_existing_directory() {
        let mut dir = temp_dir();
        dir.push("test_existing_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        let config = CONFIG
            .trim_start()
            .replace("%BASE_URL%", "https://example.com")
            .replace("%PROJECT_TITLE%", "Test Existing Dir")
            .replace(
                "%PROJECT_DESCRIPTION%",
                starter_project_description(InitStarter::AnswerFirst),
            )
            .replace("%CURATED_PACK_NAME%", starter_pack_name(InitStarter::AnswerFirst))
            .replace("%CURATED_PACK_SOURCE%", starter_pack_source(InitStarter::AnswerFirst))
            .replace("%PROMPT_VERSION%", starter_prompt_version(InitStarter::AnswerFirst));
        populate(&dir, "Test Existing Dir", &config, InitStarter::AnswerFirst)
            .expect("Could not populate ansorum directories");

        assert!(dir.join("config.toml").exists());
        assert!(dir.join("README.md").exists());
        assert!(dir.join("collections/packs/billing.toml").exists());
        assert!(dir.join("templates/base.html").exists());
        assert!(dir.join("templates/macros.html").exists());
        assert!(dir.join("templates/index.html").exists());
        assert!(dir.join("templates/page.html").exists());
        assert!(dir.join("content/refunds.md").exists());
        assert!(dir.join("content/_index.md").exists());
        assert!(dir.join("content/cancel.md").exists());
        assert!(dir.join("content/internal-playbook.md").exists());
        assert!(dir.join("content/refunds.schema.json").exists());
        assert!(dir.join("eval/fixtures.yaml").exists());
        assert!(dir.join("static").exists());
        assert!(dir.join("static/site.css").exists());
        assert!(dir.join("content").exists());
        assert!(read_file(&dir.join("config.toml")).unwrap().contains("[ansorum.redirects]"));
        assert!(read_file(&dir.join("config.toml")).unwrap().contains("generate_sitemap = true"));
        assert!(read_file(&dir.join("templates/base.html")).unwrap().contains("answers.json"));
        assert!(
            read_file(&dir.join("templates/macros.html"))
                .unwrap()
                .contains("machine_markdown_url")
        );
        assert!(read_file(&dir.join("static/site.css")).unwrap().contains("--paper"));
        assert!(read_file(&dir.join("eval/fixtures.yaml")).unwrap().contains("expected_ids"));

        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn populate_non_existing_directory() {
        let mut dir = temp_dir();
        dir.push("test_non_existing_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        let config = CONFIG
            .trim_start()
            .replace("%BASE_URL%", "https://example.com")
            .replace("%PROJECT_TITLE%", "Test Non Existing Dir")
            .replace(
                "%PROJECT_DESCRIPTION%",
                starter_project_description(InitStarter::AnswerFirst),
            )
            .replace("%CURATED_PACK_NAME%", starter_pack_name(InitStarter::AnswerFirst))
            .replace("%CURATED_PACK_SOURCE%", starter_pack_source(InitStarter::AnswerFirst))
            .replace("%PROMPT_VERSION%", starter_prompt_version(InitStarter::AnswerFirst));
        populate(&dir, "Test Non Existing Dir", &config, InitStarter::AnswerFirst)
            .expect("Could not populate ansorum directories");

        assert!(dir.exists());
        assert!(dir.join("config.toml").exists());
        assert!(dir.join("README.md").exists());
        assert!(dir.join("collections/packs/billing.toml").exists());
        assert!(dir.join("templates/base.html").exists());
        assert!(dir.join("templates/macros.html").exists());
        assert!(dir.join("templates/index.html").exists());
        assert!(dir.join("templates/page.html").exists());
        assert!(dir.join("content").exists());
        assert!(dir.join("content/_index.md").exists());
        assert!(dir.join("content/refunds.md").exists());
        assert!(dir.join("content/refunds.schema.json").exists());
        assert!(dir.join("eval/fixtures.yaml").exists());
        assert!(dir.join("static/site.css").exists());

        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn populate_ai_reference_layer_directory() {
        let mut dir = temp_dir();
        dir.push("test_ai_reference_layer_dir");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        let config = CONFIG
            .trim_start()
            .replace("%BASE_URL%", "https://example.com")
            .replace("%PROJECT_TITLE%", "AI Reference Layer")
            .replace(
                "%PROJECT_DESCRIPTION%",
                starter_project_description(InitStarter::AiReferenceLayer),
            )
            .replace("%CURATED_PACK_NAME%", starter_pack_name(InitStarter::AiReferenceLayer))
            .replace("%CURATED_PACK_SOURCE%", starter_pack_source(InitStarter::AiReferenceLayer))
            .replace("%PROMPT_VERSION%", starter_prompt_version(InitStarter::AiReferenceLayer));
        populate(&dir, "AI Reference Layer", &config, InitStarter::AiReferenceLayer)
            .expect("Could not populate AI reference layer starter");

        assert!(dir.join("collections/packs/reference-layer.toml").exists());
        assert!(dir.join("content/definitions/ai-brand-alignment.md").exists());
        assert!(dir.join("content/playbooks/improve-ai-citations.md").exists());
        assert!(dir.join("content/methodology/reference-layer-methodology.md").exists());
        assert!(dir.join("content/metrics/citation-coverage.md").exists());
        assert!(dir.join("content/comparisons/ai-reference-layer-vs-docs-portal.md").exists());
        assert!(dir.join("content/case-studies/billing-reference-case-study.md").exists());
        assert!(
            read_file(&dir.join("README.md"))
                .unwrap()
                .contains("ai-reference-layer")
        );

        remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn project_title_uses_directory_name() {
        assert_eq!(project_title(Path::new("customer-answers")), "Customer Answers");
        assert_eq!(title_case_slug("internal_support"), "Internal Support");
    }

    #[test]
    fn strip_unc_test() {
        let mut dir = temp_dir();
        dir.push("new_project1");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");
        if cfg!(target_os = "windows") {
            let stripped_path = strip_unc(&canonicalize(Path::new(&dir)).unwrap());
            assert!(same_file::is_same_file(Path::new(&stripped_path), &dir).unwrap());
            assert!(!stripped_path.starts_with(LOCAL_UNC), "The path was not stripped.");
        } else {
            assert_eq!(
                strip_unc(&canonicalize(Path::new(&dir)).unwrap()),
                canonicalize(Path::new(&dir)).unwrap().to_str().unwrap().to_string()
            );
        }

        remove_dir_all(&dir).unwrap();
    }

    // If the following test fails it means that the canonicalize function is fixed and strip_unc
    // function/workaround is not anymore required.
    // See issue https://github.com/rust-lang/rust/issues/42869 as a reference.
    #[test]
    #[cfg(target_os = "windows")]
    fn strip_unc_required_test() {
        let mut dir = temp_dir();
        dir.push("new_project2");
        if dir.exists() {
            remove_dir_all(&dir).expect("Could not free test directory");
        }
        create_dir(&dir).expect("Could not create test directory");

        let canonicalized_path = canonicalize(Path::new(&dir)).unwrap();
        assert!(same_file::is_same_file(Path::new(&canonicalized_path), &dir).unwrap());
        assert!(canonicalized_path.to_str().unwrap().starts_with(LOCAL_UNC));

        remove_dir_all(&dir).unwrap();
    }
}
