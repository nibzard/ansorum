# Ansorum Specification

Status: Draft v0.1

Repository: `https://github.com/nibzard/ansorum`

Product name: `ansorum`

Primary domain: `https://ansorum.com`

## Summary

This fork should evolve into `ansorum`, an unapologetically answer-first
compiler for agent-readable and human-readable knowledge delivery.

The product is not "a lighter Astro" and not a traditional CMS. The product is:

- Git-native content storage
- typed Markdown answer units
- a compiler that emits human and machine views of the same corpus
- a lightweight delivery gateway for Markdown negotiation and redirects
- an audit and evaluation loop for answer quality and freshness

The recommended v0 strategy is to reuse as much of Zola's implementation as is
useful, while prioritizing Ansorum's product shape over upstream compatibility.

## Product Definition

### Problem

Most current "AI-friendly docs" solutions are bolt-ons:

- `llms.txt` generators
- Markdown export plugins
- hosted docs products with partial agent support
- ad hoc search pages or chat overlays

They do not treat the answer corpus itself as the primary product surface.

### Product Thesis

The system should compile an authoritative answer graph into multiple
machine-readable and human-readable outputs from the same source corpus.

The source of truth is:

- a Git repository
- one Markdown file per answerable unit
- typed metadata describing intent, entity, audience, visibility, freshness, and
  machine-priority

### Ideal Customers

Ansorum is not for every site owner. The strongest early customers are teams
whose content is already operationally important and increasingly consumed by
agents.

Primary early customers:

- docs and developer relations teams
- support and help center owners
- product marketing and product operations teams
- AI product teams that need an authoritative answer corpus behind assistants,
  copilots, support bots, or retrieval systems

The best first customers usually have these traits:

- content already lives in Git or can be moved there
- multiple answer channels already exist and drift from each other
- mistakes in public answers create cost, risk, or support load
- they care about agent consumption, not just human pageviews
- they need governance, freshness, and auditability

Poor-fit customers:

- teams that primarily need a website builder
- teams that want a CMS UI first
- teams that do not care whether agents or LLM systems consume their content
- teams with purely marketing-site needs and little answer infrastructure

### Buyer and User Roles

Ansorum has distinct buyers, operators, and downstream users.

Economic buyers:

- head of docs
- support leadership
- developer platform leadership
- product or AI platform leadership

Primary users:

- technical writers
- docs engineers
- support content owners
- product operations
- AI/retrieval engineers

Downstream consumers:

- customers
- developers
- support agents
- internal operators
- external AI systems
- internal assistants and copilots

### Jobs To Be Done

The core JTBD is not "publish documentation." The core JTBD is:

"When our company has authoritative answers spread across docs, help centers,
internal playbooks, and product pages, help us compile them into a governed,
agent-consumable answer system so humans and machines can reliably get the right
answer."

Primary functional jobs:

- compile one authoritative answer corpus into all required delivery formats
- expose answers cleanly to both humans and agent systems
- control which answers are visible to AI systems and at what fidelity
- detect stale, duplicate, conflicting, or weak answers before publishing
- measure whether the corpus actually produces good answers in practice

Primary emotional jobs:

- reduce fear that AI systems will misrepresent the company
- reduce editorial uncertainty about which page is canonical
- give teams confidence that publishing changes will not silently degrade answer
  quality

Primary social jobs:

- let docs, support, product, and AI teams align on one canonical answer layer
- give leadership confidence that AI-facing knowledge is governed and auditable

### Core JTBD Statements

1. When I publish product, policy, or support knowledge, help me make it
   authoritative once and reusable everywhere.
2. When agents or copilots consume our content, help me decide exactly what they
   see, what they do not see, and how much context they get.
3. When we ship content changes, help me know whether answer quality improved or
   regressed before those changes reach users.
4. When our knowledge base grows, help me prevent duplicate, stale, conflicting,
   or ambiguous answers from accumulating.
5. When leadership asks whether our AI-facing knowledge is safe to expose, help
   me prove it with structure, audits, and evaluations.

### Value Proposition

Ansorum's value proposition is:

"Turn knowledge content into an answer system that is authoritative, governed,
agent-readable, and testable."

The practical customer value is not just better output formats. It is:

- one canonical source instead of many drifting answer surfaces
- lower support and operational cost from inconsistent answers
- safer AI exposure through explicit visibility and fidelity controls
- faster publishing because machine outputs are compiled, not hand-maintained
- higher confidence because answer quality is audited and evaluated
- easier integration because the system is Git-native and static-first

### Why Customers Switch

Customers adopt Ansorum when existing stacks force them to choose between:

- good human docs and weak machine outputs
- machine outputs and weak governance
- fast publishing and low confidence
- static simplicity and no evaluation loop

Ansorum should win by offering all four together:

- static simplicity
- answer-first modeling
- agent delivery
- governance and eval

### Competitive Wedge

The wedge is not "supports `llms.txt`."

That is table stakes.

The wedge is:

- answer-first authoring rather than page-first authoring
- multiple machine outputs from one canonical answer model
- built-in governance for AI visibility
- built-in audit and eval loops
- Git-native operation without a heavy app platform

### Promised Outcomes

If Ansorum is working, a customer should be able to say:

- "We know which answer is canonical."
- "We can expose content to agents without exposing everything."
- "We can ship machine-readable answers without maintaining parallel content."
- "We can detect answer regressions before they ship."
- "Our docs, support, and AI teams are operating on the same answer layer."

### Non-Goals

This project should not start by building:

- a WYSIWYG editor
- a database-backed admin
- a theme marketplace
- a React app shell
- an on-site chat product
- autonomous direct-to-main agent editing

These can be added later if justified. They are not part of the core product.

## Product Decisions

These decisions are fixed unless explicitly revised later.

- Name: `ansorum`
- Positioning: answer-first product, not a general-purpose SSG
- Compatibility stance: move fast on the new product shape over preserving
  upstream behavior
- Answer metadata: first-class front matter immediately
- Missing answer metadata: build errors by default
- AI visibility: `summary_only` exposes canonical links to hidden full pages
- Structured data: broad support immediately
- Evaluation: LLM-based scoring from the start
- Evaluation backend: OpenAI Responses API with GPT-5.4 models first
- Analytics: logs and event hooks only in v0
- Redirects: external redirects allowed with allowlist enforcement
- Canonical machine Markdown route: `/page.md`
- Structured data authoring: sidecar files
- CLI compatibility: drop `zola` alias

## Guiding Principles

1. Git is the CMS.
2. Markdown is the authoring format.
3. Each answerable unit is independently addressable.
4. Published machine Markdown reflects rendered content, not raw source only.
5. `llms.txt` is an output format, not the internal model.
6. Analytics and evaluation are first-class, not afterthoughts.
7. Public delivery must support both human HTML and machine Markdown cleanly.

## High-Level Architecture

The system is composed of five layers:

1. Content model
2. Compiler
3. Output emitters
4. Delivery gateway
5. Audit and evaluation tooling

For v0, these still map onto the current codebase as follows:

- content parsing: `components/content`
- Markdown rendering: `components/markdown`
- templates and global functions: `components/templates`
- site build orchestration: `components/site`
- CLI command surface: `src/cmd`
- local server: `src/cmd/serve.rs`

## Content Model

### Unit of Content

The atomic unit is an answer, not a section page.

An answer may represent:

- concept
- task
- policy
- troubleshooting step
- comparison
- pricing rule
- integration guide
- FAQ item
- reference entry

Each answer should be stored in one Markdown file.

### v0 Storage Model

For v0, answers still live as files under `content/`, but answer metadata is
not treated as generic extra data.

Ansorum should introduce first-class answer front matter immediately.

This means:

- one file in `content/` equals one answer unit
- section hierarchy may still exist for organization
- answer semantics are defined by explicit answer fields, not inferred only from
  directory structure
- upstream compatibility is secondary to a clean answer-first model

### Front Matter Schema

Authors may use TOML or YAML front matter as currently supported by Zola.

Answer metadata should be first-class front matter fields.

Example:

```toml
+++
title = "Refund policy"
description = "How refunds work, eligibility, timing, and exceptions."

id = "refunds-policy"
summary = "How refunds work, eligibility, timing, and exceptions."
canonical_questions = [
  "how do refunds work",
  "can i get a refund",
  "refund eligibility",
]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["cancellations", "pricing"]
external_refs = ["https://stripe.com/docs/..."]
schema_type = "WebPage"
review_by = "2026-06-01"
priority = "high"
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
aliases = ["refund policy", "refunds", "refund rules"]
+++
```

### Required Answer Fields

The compiler should require these for answer pages:

- `id`
- `summary`
- `intent`
- `entity`
- `audience`
- `visibility`
- `ai_visibility`
- `llms_priority`
- `token_budget`

At least one of these must be present:

- `canonical_questions`
- `title`

### Optional Answer Fields

- `related`
- `external_refs`
- `review_by`
- `priority`
- `schema_type`
- `aliases`
- `ai_extra`
- `last_reviewed_by`
- `owner`
- `confidence_notes`

### Controlled Vocabularies

The compiler should validate these enums.

`intent`:

- `concept`
- `task`
- `policy`
- `troubleshooting`
- `comparison`
- `pricing`
- `integration`
- `faq`
- `reference`

`audience`:

- `customer`
- `prospect`
- `developer`
- `admin`
- `internal`

`visibility`:

- `public`
- `private`
- `internal`

`ai_visibility`:

- `public`
- `hidden`
- `summary_only`

`llms_priority`:

- `core`
- `optional`
- `hidden`

`token_budget`:

- `small`
- `medium`
- `full`

### First-Class Answer Type

The codebase should introduce a dedicated answer front matter model rather than
continuing to treat answer metadata as untyped extra fields.

Suggested shape:

```rust
pub struct AnswerFrontMatter {
    pub id: String,
    pub summary: String,
    pub canonical_questions: Vec<String>,
    pub intent: AnswerIntent,
    pub entity: String,
    pub audience: AnswerAudience,
    pub related: Vec<String>,
    pub external_refs: Vec<String>,
    pub schema_type: Option<String>,
    pub review_by: Option<String>,
    pub priority: Option<String>,
    pub visibility: AnswerVisibility,
    pub ai_visibility: AiVisibility,
    pub llms_priority: LlmsPriority,
    pub token_budget: TokenBudget,
    pub aliases: Vec<String>,
    pub ai_extra: Option<String>,
}
```

### Relationship Model

The internal answer graph should support:

- related answers
- same-entity answers
- same-intent answers
- same-audience answers
- aliases and canonical question sets

For v0, these relationships can be derived from:

- `related`
- front matter metadata
- explicit pack definitions
- optional taxonomies where they remain useful for organization

No separate graph store is required yet.

## URL and Output Model

### Canonical Page Outputs

Each answer page should emit:

- canonical HTML
- canonical machine Markdown
- JSON-LD

Example:

- `/refunds/`
- `/refunds.md`
- structured data embedded in HTML and optionally emitted as sidecar JSON

### Global Machine Outputs

The compiler must emit:

- `/llms.txt`
- `/llms-full.txt`
- `/answers.json`
- `/sitemap.xml`

### Topic or Entity Packs

The compiler should emit scoped machine bundles such as:

- `/billing/llms.txt`
- `/billing/answers.json`
- `/developers/llms.txt`
- `/pricing/llms.txt`

These packs are subsets of the global corpus.

Ansorum should generate scoped packs using a mixed strategy:

- automatic packs by `entity`
- automatic packs by `audience`
- optional curated packs from `collections/packs/*.toml`

This gives predictable defaults while allowing editorial control for important
curated domains like billing, pricing, onboarding, or enterprise.

### Markdown Delivery Model

The delivery layer must support both:

- explicit Markdown routes
- content negotiation

Supported patterns:

- `/page.md`
- `/page` with `Accept: text/markdown`

The server must set:

- `Content-Type: text/markdown; charset=utf-8` for Markdown responses
- `Vary: Accept` on negotiated routes

### Redirect Model

Tracking and attribution codes must use path-based routes, not fragments.

Canonical pattern:

- `/r/:code`

Examples:

- `/r/42ascpo999s`
- `/r/sales-demo`

The system should log redirect events server-side before forwarding users to the
resolved destination.

## Compiler Outputs

### 1. HTML

Human-readable pages remain normal Zola-rendered pages.

Requirements:

- existing template support remains intact
- rendered HTML remains canonical for humans
- JSON-LD is embedded in page output

### 2. Machine Markdown

Machine Markdown must be derived from the rendered content model, not exposed
blindly from raw source.

Requirements:

- preserve computed content that exists after rendering
- preserve normalized headings, links, and extracted answer metadata
- avoid leaking author-only implementation details
- optionally omit hidden blocks based on AI visibility rules

### 3. `llms.txt`

The compiler must generate a curated root `llms.txt`.

It should include:

- project title
- short corpus description
- core answer links
- optional section for lower-priority context
- references to scoped packs where relevant

### 4. `llms-full.txt`

The compiler must generate a broad export of the public AI-visible corpus.

It should include:

- all `core` entries
- all `optional` entries allowed for AI visibility
- stable ordering
- concise summaries
- canonical links to machine Markdown pages

### 5. `answers.json`

This is the canonical machine index.

Each record should include:

- `id`
- `title`
- `summary`
- `canonical_url`
- `markdown_url`
- `entity`
- `intent`
- `audience`
- `related`
- `canonical_questions`
- `aliases`
- `review_by`
- `llms_priority`
- `token_budget`
- `visibility`
- `ai_visibility`
- `last_modified`

Example shape:

```json
{
  "version": 1,
  "generated_at": "2026-03-18T00:00:00Z",
  "answers": [
    {
      "id": "refunds-policy",
      "title": "Refund policy",
      "summary": "How refunds work, eligibility, timing, and exceptions.",
      "canonical_url": "https://example.com/refunds/",
      "markdown_url": "https://example.com/refunds.md",
      "entity": "billing",
      "intent": "policy",
      "audience": "customer",
      "canonical_questions": ["how do refunds work"],
      "aliases": ["refund policy"],
      "related": ["cancellations", "pricing"],
      "llms_priority": "core",
      "token_budget": "medium",
      "visibility": "public",
      "ai_visibility": "public"
    }
  ]
}
```

### 6. JSON-LD

Each page should emit JSON-LD with schema appropriate to its `schema_type`.

Ansorum should support arbitrary structured data immediately.

That means:

- common built-in schema presets should exist
- authors may provide explicit JSON-LD payloads in sidecar files where needed
- unsupported schema types must not require core code changes

Recommended authoring pattern:

- `content/billing/refunds.md`
- `content/billing/refunds.schema.json`

Canonical sidecar naming convention for v0:

- exactly one structured-data sidecar may exist per answer source file
- the sidecar must be a sibling file whose basename matches the Markdown file
  stem exactly
- the filename pattern is `<answer-file-stem>.schema.json`
- examples:
  - `refunds.md` -> `refunds.schema.json`
  - `refunds.en.md` -> `refunds.en.schema.json`

Built-in presets should cover at least:

- `WebPage`
- `FAQPage`
- `HowTo`
- `Article`
- `TechArticle`
- `DefinedTerm`
- `Product`
- `SoftwareApplication`
- `Organization`
- `Service`
- `Offer`
- `BreadcrumbList`

The long-term stance is open-ended schema support, not a closed enum.

## Delivery Gateway Specification

### Responsibilities

The gateway is intentionally small. It is not a full app framework.

Responsibilities:

- serve static files
- negotiate HTML vs Markdown
- return machine outputs
- serve `llms.txt` variants
- handle `/r/:code` redirects
- emit analytics events

### Local Development

The development server may continue to use the existing Axum-based serve path in
`src/cmd/serve.rs`.

V0 changes should add:

- Markdown route resolution
- `Accept: text/markdown` negotiation
- `Vary: Accept`
- redirect route handling

### Caching Rules

Negotiated routes must be cache-safe.

Requirements:

- Markdown and HTML variants must not be conflated by caches
- negotiated routes must use `Vary: Accept`
- explicit `.md` routes may be cached independently

## Authoring Model

### Repo Layout

Recommended v0 layout:

```text
content/
  billing/
    refunds.md
    cancellations.md
  pricing/
    overview.md
  integrations/
    stripe.md
templates/
  page.html
  section.html
  answer_markdown.txt
  llms.txt
  llms_full.txt
  answers.json
  structured_data.json
collections/
  packs/
    billing.toml
    pricing.toml
static/
schemas/
```

### Editorial Rules

Each answer should:

- answer one primary intent cleanly
- include a concise summary
- declare canonical questions
- link to related answers explicitly
- include a review date if high priority
- use stable IDs independent of URL path

### AI Visibility Rules

Three levels must exist:

- `public`: appears in machine outputs
- `summary_only`: appears in indexes with summary and canonical links, but full
  AI-facing content excluded
- `hidden`: excluded from AI-facing outputs entirely

Human visibility and AI visibility are separate concerns.

## Command Surface

The long-term product should expose these commands:

- `ansorum init`
- `ansorum build`
- `ansorum serve`
- `ansorum audit`
- `ansorum eval`

### v0 Command Mapping

For the forked Zola codebase, the fastest route is:

- extend `build`
- extend `serve`
- extend `check`
- add new command(s) incrementally

Recommended near-term CLI:

- `ansorum build`
- `ansorum serve`
- `ansorum audit`
- `ansorum eval`
- `ansorum init`

## Build Pipeline Specification

### Current Base

Today the build pipeline is:

1. load config
2. parse pages and sections
3. populate taxonomies and relationships
4. render Markdown
5. build HTML outputs and assets

### v0 Extended Pipeline

The new build pipeline should be:

1. load config
2. parse pages and sections
3. extract first-class answer metadata
4. validate answer schema
5. derive answer graph relationships
6. render Markdown to HTML
7. render machine Markdown exports
8. emit `answers.json`
9. emit `llms.txt` and scoped packs
10. emit JSON-LD
11. emit sitemap and existing outputs
12. write audit manifest

### Internal Compiler Types

Introduce a normalized internal type such as:

```rust
pub struct AnswerRecord {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub canonical_url: String,
    pub markdown_url: String,
    pub intent: String,
    pub entity: String,
    pub audience: String,
    pub canonical_questions: Vec<String>,
    pub aliases: Vec<String>,
    pub related: Vec<String>,
    pub visibility: String,
    pub ai_visibility: String,
    pub llms_priority: String,
    pub token_budget: String,
    pub review_by: Option<String>,
}
```

This should be derived from authored answer pages, but it should become a core
compiler type rather than a thin compatibility shim.

## Audit Specification

The audit layer is core product functionality.

### Checks

`audit` should report:

- missing required answer fields
- duplicate answer IDs
- duplicate canonical questions
- duplicate aliases across different answers
- stale `review_by` dates
- missing summaries
- missing related links for high-priority answers
- invalid enum values
- missing JSON-LD type
- token budget overflow
- hidden content leaking into AI outputs

### Severity Levels

By default, schema and answer-metadata failures should stop the build.

Each finding should be:

- `error`
- `warn`
- `info`

### Output Formats

Audit should support:

- human-readable terminal output
- JSON output for CI

## Evaluation Specification

The eval layer tests whether the corpus answers real questions correctly.

### Inputs

Evaluation data should live in version-controlled fixtures.

Example:

```yaml
- question: can i get a refund after 30 days
  expected_ids: [refunds-policy]
  forbidden_ids: [trial-cancellation]
  required_terms: [eligibility, exceptions]
```

### v0 Eval Modes

1. retrieval eval
2. answer selection eval
3. LLM-scored answer quality eval

Retrieval eval checks whether expected answer IDs are surfaced from the answer
index.

Answer selection eval checks whether the rendered machine view contains the
required concepts for a benchmark question.

LLM-scored eval uses a configured model to grade:

- relevance
- completeness
- factual consistency against expected answer IDs
- citation or canonical-link quality
- preference ordering when multiple answers compete

The first required backend is:

- OpenAI Responses API
- GPT-5.4 family models

### Required v0 Eval Properties

- deterministic fixture inputs
- machine-readable results
- model and prompt version stamping
- reproducible score reports
- threshold-based pass/fail support in CI

## Analytics Specification

Analytics must capture both human and machine behavior.

### Events

Track at minimum:

- HTML page view
- Markdown fetch
- negotiated Markdown fetch
- `llms.txt` fetch
- `llms-full.txt` fetch
- scoped pack fetch
- `/r/:code` redirect hit
- feedback vote
- copy answer action
- low-confidence search exit

### Data Sinks

V0 should emit:

- structured server logs
- event hooks

V0 should not depend on external analytics services.

### Principles

- analytics must be optional
- no secret-dependent build behavior
- privacy-sensitive data must remain configurable

## Security and Safety

### Authoring Safety

Do not allow direct autonomous agent writes to main by default.

Recommended workflow:

- agent proposes changes in branch or PR
- audit runs in CI
- eval runs in CI
- human review merges

### Output Safety

Machine outputs must respect:

- `visibility`
- `ai_visibility`
- hidden partials and excluded blocks

### Redirect Safety

Redirect targets must be validated:

- internal redirects allowed by default
- external redirects allowed only when the target host matches an allowlist
- disallowed targets fail build or config validation

## Implementation Plan for This Fork

### Phase 1: Metadata and Internal Model

Add first-class answer front matter parsing and validation.

Suggested modules:

- `components/content/src/answer.rs`
- `components/site/src/answers.rs`
- `components/content/src/front_matter/answer.rs`

Responsibilities:

- parse answer front matter
- validate required fields
- build normalized `AnswerRecord` values

### Phase 2: Emitters

Add emitters for:

- `answers.json`
- `llms.txt`
- `llms-full.txt`
- scoped packs
- per-page Markdown exports

Suggested modules:

- `components/site/src/answers.rs`
- `components/site/src/llms.rs`
- templates for text and JSON serialization where appropriate

### Phase 3: Delivery

Extend `src/cmd/serve.rs` to:

- resolve `.md` requests
- negotiate `Accept: text/markdown`
- set `Vary: Accept`
- serve redirect routes

### Phase 4: Audit

Add a new command:

- `src/cmd/audit.rs`

And wire it through:

- `src/cli.rs`
- `src/cmd/mod.rs`
- `src/main.rs`

### Phase 5: Eval

Add:

- `src/cmd/eval.rs`

This should load fixture questions, validate the compiled corpus, and support
LLM-based rubric scoring from the beginning.

## Migration Strategy

### v0

Keep the filesystem content model, but do not preserve Zola's front matter shape
when it conflicts with Ansorum's answer-first design.

Use:

- first-class answer front matter
- existing content tree where useful
- existing template/render pipeline where useful

### v1

Add first-class internal answer graph APIs while preserving filesystem-backed
authoring.

### v2

Optionally decouple answer graph storage from section hierarchy if the Zola page
model becomes a limiting factor.

## Naming

Canonical name:

- `ansorum`

## Open Questions

Resolved for v0:

1. Structured-data sidecars use the canonical sibling naming pattern
   `<answer-file-stem>.schema.json`.
2. LLM eval defaults to `gpt-5.4-mini` when no explicit model override is
   provided.

## Success Criteria for v0

v0 is successful if this fork can:

1. compile a Markdown corpus with first-class typed answer metadata
2. emit HTML and machine Markdown from the same content
3. generate `llms.txt`, `llms-full.txt`, and `answers.json`
4. serve negotiated Markdown correctly
5. audit schema and freshness issues in CI
6. evaluate a benchmark question set against the compiled corpus

## Bottom Line

This fork should be developed as:

Git repo + typed Markdown + compiler + gateway + audit/eval loop

not:

another general-purpose website framework.

The fastest path is to reuse the existing implementation selectively while
shaping the codebase around Ansorum's answer-first model immediately.
