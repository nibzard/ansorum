+++
title = "Reference Project"
weight = 15
+++

## Reference Ansorum Project

Ansorum's canonical answer-first example lives in `examples/reference-project/`.

It is intentionally small, but it exercises the full v0 workflow in one coherent
corpus:

- public billing answers with first-class answer frontmatter
- both TOML and YAML frontmatter styles
- internal-only support content with hidden AI visibility
- canonical markdown output at `/page.md`
- machine indexes via `answers.json`, `llms.txt`, `llms-full.txt`, and scoped packs
- search output via `search_index.en.js`
- JSON-LD sidecars from `<answer-stem>.schema.json`
- configured `/r/:code` redirects
- deterministic `audit` and `eval` inputs
- governance metadata such as `review_by`, `owner`, and `confidence_notes`

## Why This Project Matters

Use this project as the source of truth for:

- how to author first-class answer frontmatter
- when to use `public`, `summary_only`, and `hidden` AI visibility
- how sidecar JSON-LD files are named and placed
- how packs and redirects are configured
- what `audit` and `eval` expect from a governed answer corpus

## Run The Workflow

From the repository root:

```bash
cd examples/reference-project
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

`ansorum eval` runs the deterministic retrieval and answer-selection checks in
`eval/fixtures.yaml`. Add `--llm` only when you have an `OPENAI_API_KEY` and
want OpenAI Responses API grading.

## Authoring Patterns To Copy

The reference project deliberately shows both supported frontmatter styles:

- `content/refunds.md` and `content/cancel.md` use TOML frontmatter
- `content/billing-credits.md` uses YAML frontmatter

It also demonstrates the core policy controls:

- `ai_visibility = "public"` for fully machine-readable public answers
- `ai_visibility = "summary_only"` when agents should get the summary and
  canonical links but not the full rendered body
- `ai_visibility = "hidden"` when an answer should stay out of machine outputs
- `review_by`, `owner`, and `confidence_notes` as governance-oriented metadata
  that `audit` can reason about

The pack setup is intentionally selective:

- curated packs are enabled
- audience packs are enabled
- entity packs are left disabled to keep the example output surface compact

Structured data is authored as a sibling file, for example
`content/refunds.schema.json`. Ansorum's v0 convention is one sidecar per
answer, named as `<answer-stem>.schema.json`.

## What To Inspect

After `ansorum build`, the reference project should produce:

- `public/refunds/page.md`
- `public/refunds/schema.json`
- `public/cancel/page.md`
- `public/cancel/schema.json`
- `public/answers.json`
- `public/llms.txt`
- `public/llms-full.txt`
- `public/billing/llms.txt`
- `public/billing/answers.json`
- `public/customer/llms.txt`
- `public/customer/answers.json`
- `public/search_index.en.js`
- `public/legacy/refund-policy/index.html`

During `ansorum serve`, the same project demonstrates:

- canonical HTML at `/refunds/`
- negotiated Markdown with `Accept: text/markdown`
- explicit Markdown at `/refunds/page.md`
- `summary_only` Markdown behavior at `/cancel/page.md`
- redirect delivery at `/r/sales-demo` and `/r/billing-portal`

Treat this project as the source of truth for screenshots, docs examples, and
future contributor onboarding.
