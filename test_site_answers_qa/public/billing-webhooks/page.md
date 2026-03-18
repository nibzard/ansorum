---
id: billing-webhooks
canonical_url: https://answers.example.com/billing-webhooks/
visibility: public
ai_visibility: public
intent: reference
entity: api
audience: customer
llms_priority: optional
token_budget: medium
canonical_questions:
- billing webhooks
- invoice event payload
- payment failed webhook
retrieval_aliases:
- invoice webhook
- billing events
- payment webhook
related:
- payment-recovery
- invoice-download
- usage-credits
---

# Billing webhooks and invoice events

Which billing events are emitted, what payloads look like, and how to build idempotent handlers.

The billing system emits signed events for invoice lifecycle changes, payment
retries, and credit balance updates.

## Event types

- `invoice.finalized`
- `invoice.payment_failed`
- `invoice.paid`
- `credits.balance_updated`

## Handler guidance

- Verify the signature before parsing the payload.
- Use the event ID as the idempotency key.
- Treat delivery as at-least-once and safe to replay.

## Example payload

{
  &quot;id&quot;: &quot;evt_01JXYZ7M5N6PQ&quot;,
  &quot;type&quot;: &quot;invoice.payment_failed&quot;,
  &quot;created_at&quot;: &quot;2026-03-18T11:04:00Z&quot;,
  &quot;data&quot;: {
    &quot;invoice_id&quot;: &quot;inv_2026_000184&quot;,
    &quot;workspace&quot;: &quot;acme&quot;,
    &quot;attempt&quot;: 2,
    &quot;status&quot;: &quot;past_due&quot;
  }
}
## Failure modes to test

- Signature mismatch
- Duplicate delivery
- Out-of-order retries after a manual payment succeeds
