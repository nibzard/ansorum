+++
title = "Manage team billing roles and approvals"
weight = 50

id = "team-billing-admin"
summary = "Who can view invoices, who can change plans, and how approval workflows typically work."
canonical_questions = ["who can manage billing", "billing admin permissions", "team approval workflow"]
intent = "reference"
entity = "billing"
audience = "customer"
related = ["invoice-download", "cancel-subscription", "payment-recovery"]
external_refs = ["https://example.com/roles"]
schema_type = "Article"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["billing roles", "billing admin", "owner permissions"]
+++

Billing workflows are safest when invoice access and plan changes are scoped to
the right roles instead of shared credentials.

## Role matrix

| Role | View invoices | Update payment method | Change plan | Cancel subscription |
| --- | --- | --- | --- | --- |
| Owner | Yes | Yes | Yes | Yes |
| Billing admin | Yes | Yes | Yes | No |
| Admin | No | No | No | No |
| Member | No | No | No | No |

## Recommended approval model

- Give finance users the billing admin role.
- Keep ownership with a technical or operations lead.
- Require an internal approval ticket for annual-plan upgrades and workspace cancellations.

## Audit habits

- Review role assignments monthly.
- Remove billing access during offboarding on the same day.
- Avoid forwarding invoices to personal email aliases.
