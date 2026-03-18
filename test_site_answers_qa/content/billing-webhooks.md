+++
title = "Billing webhooks and invoice events"
weight = 80

id = "billing-webhooks"
summary = "Which billing events are emitted, what payloads look like, and how to build idempotent handlers."
canonical_questions = ["billing webhooks", "invoice event payload", "payment failed webhook"]
intent = "reference"
entity = "api"
audience = "customer"
related = ["payment-recovery", "invoice-download", "usage-credits"]
external_refs = ["https://example.com/webhooks"]
schema_type = "Article"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["invoice webhook", "billing events", "payment webhook"]
+++

The billing system emits signed events for invoice lifecycle changes, payment
retries, and credit balance updates.

## Event types

- `invoice.finalized`
- `invoice.payment_failed`
- `invoice.paid`
- `credits.balance_updated`

## Handler guidance

1. Verify the signature before parsing the payload.
2. Use the event ID as the idempotency key.
3. Treat delivery as at-least-once and safe to replay.

## Example payload

```json
{
  "id": "evt_01JXYZ7M5N6PQ",
  "type": "invoice.payment_failed",
  "created_at": "2026-03-18T11:04:00Z",
  "data": {
    "invoice_id": "inv_2026_000184",
    "workspace": "acme",
    "attempt": 2,
    "status": "past_due"
  }
}
```

## Failure modes to test

- Signature mismatch
- Duplicate delivery
- Out-of-order retries after a manual payment succeeds
