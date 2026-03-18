+++
title = "How do I cancel my subscription?"
weight = 20

id = "cancel-subscription"
summary = "How to cancel a subscription, what happens to access, and where to confirm the change."
canonical_questions = ["how do i cancel my subscription", "stop auto renewal", "cancel workspace plan"]
intent = "task"
entity = "billing"
audience = "customer"
related = ["refunds-policy", "invoice-download", "team-billing-admin"]
external_refs = ["https://example.com/billing"]
schema_type = "HowTo"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "summary_only"
llms_priority = "optional"
token_budget = "small"
retrieval_aliases = ["cancel subscription", "turn off renewal", "end plan"]
aliases = ["legacy/cancel-flow"]
+++

Canceling a paid plan stops the next renewal. It does **not** delete the
workspace and it does **not** retroactively refund already-consumed usage.

## Before you cancel

- Only workspace owners can cancel paid subscriptions.
- Seat removals and plan downgrades do not take effect instantly on invoices that are already finalized.
- If you need account history first, download your receipts before canceling.

## Steps

1. Open the billing portal from **Settings -> Billing**.
2. Select **Manage subscription**.
3. Choose **Cancel plan** and confirm the effective date shown in the portal.
4. Verify that the workspace now shows a scheduled cancellation banner.

## What happens next

- Access continues through the paid-through date.
- Members keep existing data unless the workspace is separately deleted.
- Future invoices stop after the cancellation effective date.

## Common edge cases

### I cannot see the cancel button

This usually means your role is not owner, the workspace is on invoiced terms,
or the subscription is managed through a reseller.

### I canceled but still got charged

Check whether the cancellation effective date was after the renewal job cut-off.
If so, review [Refund policy](/refunds/) and share the invoice ID with support.
