# Reference Ansorum Project

This directory is the canonical answer-first example project for Ansorum.

It demonstrates one coherent workflow in a small billing and support corpus:

- first-class answer frontmatter in `content/*.md`
- both TOML and YAML answer frontmatter
- canonical machine markdown at `/page.md`
- structured-data sidecars via `content/<answer-stem>.schema.json`
- root and scoped machine indexes via `llms.txt`, `llms-full.txt`, and `answers.json`
- search output via `search_index.en.js`
- redirect routes configured under `[ansorum.redirects]`
- audit-ready answer metadata such as `review_by`, `owner`, and `confidence_notes`
- deterministic eval fixtures in `eval/fixtures.yaml`

Run the full workflow from this directory:

```bash
ansorum build
ansorum serve
ansorum audit
ansorum eval
```

The build should emit these reference outputs:

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

Intentional choices in this example:

- `cancel.md` keeps `ai_visibility = "summary_only"` so the corpus shows how a
  public answer can expose summary and canonical links to machines without
  exposing the full body.
- `billing-credits.md` and `internal-playbook.md` stay internal and hidden so
  the example shows governance boundaries as well as public delivery.
- `auto_entity_packs = false` keeps the output surface compact. The example
  focuses on curated packs and audience packs rather than every possible pack
  type.

Use `ansorum serve` to exercise the delivery workflow:

- `GET /refunds/` for HTML
- `GET /refunds/page.md` for canonical machine markdown
- `GET /cancel/page.md` for a `summary_only` machine view
- `GET /refunds/` with `Accept: text/markdown` for negotiated markdown
- `GET /r/sales-demo` for an external allowlisted redirect
- `GET /r/billing-portal` for a site-relative redirect

Use `ansorum eval --llm` only when `OPENAI_API_KEY` is set and you want rubric scoring.
