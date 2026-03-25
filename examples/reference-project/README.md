# Reference Ansorum Project

This directory is the canonical answer-first example project for Ansorum.

It demonstrates one coherent workflow in a small billing and support corpus:

- first-class answer frontmatter in `content/*.md`
- canonical machine markdown at `/page.md`
- structured-data sidecars via `content/<answer-stem>.schema.json`
- root and scoped machine indexes via `llms.txt`, `llms-full.txt`, and `answers.json`
- redirect routes configured under `[ansorum.redirects]`
- audit-ready answer metadata
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
- `public/answers.json`
- `public/llms.txt`
- `public/llms-full.txt`
- `public/billing/llms.txt`
- `public/billing/answers.json`
- `public/customer/llms.txt`
- `public/customer/answers.json`

Use `ansorum serve` to exercise the delivery workflow:

- `GET /refunds/` for HTML
- `GET /refunds/page.md` for canonical machine markdown
- `GET /refunds/` with `Accept: text/markdown` for negotiated markdown
- `GET /r/sales-demo` for an external allowlisted redirect
- `GET /r/billing-portal` for a site-relative redirect

Use `ansorum eval --llm` only when `OPENAI_API_KEY` is set and you want rubric scoring.
