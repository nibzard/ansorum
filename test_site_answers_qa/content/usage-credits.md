+++
title = "Usage credits and promotional balances"
weight = 70

id = "usage-credits"
summary = "How prepaid and promotional credits are applied, how they expire, and when they do not reduce invoices."
canonical_questions = ["how do credits work", "promo balance", "why were credits not applied"]
intent = "concept"
entity = "billing"
audience = "customer"
related = ["invoice-download", "refunds-policy", "payment-recovery"]
external_refs = ["https://example.com/credits"]
schema_type = "Article"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["credit balance", "promotional credits", "prepaid credits"]
+++

Credits reduce eligible usage charges before the remaining balance is invoiced.
They do not automatically offset taxes, prior-due invoices, or reseller-managed
contracts.

## Credit types

- Prepaid credits purchased through sales.
- Service credits granted for a documented outage.
- Promotional credits issued for trials or launches.

## Rules that matter in QA

- Promotional credits can expire.
- Credits are applied oldest-expiring first.
- Credits usually do not cross workspace boundaries unless contractually linked.

## Example balance payload

```json
{
  "workspace": "acme",
  "available_cents": 125000,
  "currency": "USD",
  "expiring": [
    { "amount_cents": 25000, "expires_at": "2026-09-30" }
  ]
}
```
