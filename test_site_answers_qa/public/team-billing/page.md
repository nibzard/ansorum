---
id: team-billing-admin
canonical_url: https://answers.example.com/team-billing/
visibility: public
ai_visibility: public
intent: reference
entity: billing
audience: customer
llms_priority: optional
token_budget: medium
canonical_questions:
- who can manage billing
- billing admin permissions
- team approval workflow
retrieval_aliases:
- billing roles
- billing admin
- owner permissions
related:
- invoice-download
- cancel-subscription
- payment-recovery
---

# Manage team billing roles and approvals

Who can view invoices, who can change plans, and how approval workflows typically work.

Billing workflows are safest when invoice access and plan changes are scoped to
the right roles instead of shared credentials.

## Role matrix

RoleView invoicesUpdate payment methodChange planCancel subscription
OwnerYesYesYesYes
Billing adminYesYesYesNo
AdminNoNoNoNo
MemberNoNoNoNo

## Recommended approval model

- Give finance users the billing admin role.
- Keep ownership with a technical or operations lead.
- Require an internal approval ticket for annual-plan upgrades and workspace cancellations.

## Audit habits

- Review role assignments monthly.
- Remove billing access during offboarding on the same day.
- Avoid forwarding invoices to personal email aliases.
