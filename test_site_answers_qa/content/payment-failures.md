+++
title = "What happens when a payment fails?"
weight = 40

id = "payment-recovery"
summary = "Retry timing, grace periods, and how to recover a failed subscription payment."
canonical_questions = ["payment failed", "card declined renewal", "how do retries work"]
intent = "reference"
entity = "billing"
audience = "customer"
related = ["invoice-download", "cancel-subscription", "team-billing-admin"]
external_refs = ["https://example.com/payments"]
schema_type = "FAQPage"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["failed payment", "retry schedule", "declined charge"]
+++

If a renewal payment fails, the invoice remains open and the system retries the
charge automatically before service is restricted.

## Retry schedule

| Attempt | When | Customer-facing effect |
| --- | --- | --- |
| 1 | Renewal time | Invoice opens as `past_due` |
| 2 | 24 hours later | Reminder email to billing admins |
| 3 | 72 hours later | In-app banner for owners |
| 4 | 7 days later | Service may enter read-only mode |

## Recommended recovery steps

1. Update the payment method from the billing portal.
2. Reopen the past-due invoice and click **Retry payment**.
3. Confirm the status changes to `paid` before closing the incident.

## Common causes

- Card expired or replaced after fraud reissue.
- Bank blocked the charge because of region or MCC restrictions.
- Invoice exceeded the card's transaction limit.

## If the workspace is already restricted

Support can temporarily extend the grace window for enterprise customers, but
that requires an open invoice ID and a documented owner request.
