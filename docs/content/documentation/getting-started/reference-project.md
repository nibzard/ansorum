+++
title = "Reference Project"
weight = 15
+++

## Reference Ansorum Project

Ansorum's canonical answer-first example lives in `test_site_answers/`.

It is intentionally small, but it exercises the full v0 workflow in one coherent
corpus:

- public billing answers with first-class answer frontmatter
- internal-only support content with hidden AI visibility
- canonical markdown output at `/page.md`
- machine indexes via `answers.json`, `llms.txt`, `llms-full.txt`, and scoped packs
- JSON-LD sidecars from `*.schema.json`
- configured `/r/:code` redirects
- deterministic `audit` and `eval` inputs

## Run The Workflow

From the repository root:

```bash
cd test_site_answers
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

`ansorum eval` runs the deterministic retrieval and answer-selection checks in
`eval/fixtures.yaml`. Add `--llm` only when you have an `OPENAI_API_KEY` and
want OpenAI Responses API grading.

## What To Inspect

After `ansorum build`, the reference project should produce:

- `public/refunds/page.md`
- `public/refunds/schema.json`
- `public/cancel/page.md`
- `public/answers.json`
- `public/llms.txt`
- `public/llms-full.txt`
- `public/billing/llms.txt`
- `public/customer/llms.txt`

During `ansorum serve`, the same project demonstrates:

- canonical HTML at `/refunds/`
- negotiated Markdown with `Accept: text/markdown`
- explicit Markdown at `/refunds/page.md`
- redirect delivery at `/r/sales-demo` and `/r/billing-portal`

Treat this project as the source of truth for screenshots, docs examples, and
future contributor onboarding.
