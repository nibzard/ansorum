+++
title = "Refund policy"
weight = 10

id = "refunds-policy"
summary = "How refunds work, who qualifies, and when payment returns land."
canonical_questions = ["how do refunds work", "can i get a refund", "how long do refunds take"]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["cancel-subscription", "invoice-download", "tax-exemption"]
external_refs = ["https://example.com/refunds", "https://example.com/terms"]
schema_type = "FAQPage"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["refund policy", "refund rules", "refund timing"]
aliases = ["legacy/refund-policy", "help/refunds"]
+++

Refunds are available when a duplicate charge, billing defect, or policy-backed
exception is confirmed by support. Most requests are resolved within one business
day, but settlement timing depends on the original payment rail.

## Quick answer

- Self-serve subscription cancellations stop future renewals but do not create an automatic refund.
- Annual prepayments are reviewed case by case.
- Taxes and bank fees are refunded only if the original processor returns them.

## Eligibility

You are usually eligible when one of these is true:

- You were charged twice for the same invoice.
- A seat was billed after a confirmed downgrade effective date.
- A product outage prevented access during the initial purchase window.

You are usually not eligible when:

- The workspace stayed active after renewal and usage continued.
- The request is for overage or metered usage that has already been consumed.
- The invoice is older than 60 days and there is no processor error.

## Typical timeline

| Payment method | Review SLA | Funds returned in | Notes |
| --- | --- | --- | --- |
| Card | 1 business day | 5 to 10 business days | Depends on issuer posting speed |
| ACH debit | 2 business days | 5 to 7 business days | Bank holidays can add delay |
| Wire transfer | 3 business days | 3 to 5 business days | Returned to original account only |

## What support needs

1. Workspace URL or account email.
2. Invoice number or last four digits of the charged card.
3. A short reason for the request and any screenshots of the billing issue.

## Related workflows

- If the goal is to stop future renewals, see [How do I cancel my subscription?](/cancel/).
- If finance needs paperwork, see [Download invoices and receipts](/invoices/).

> Support can approve exceptions, but billing policy still governs final outcomes.
