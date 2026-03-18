+++
title = "Sales tax, VAT, and tax exemption"
weight = 60

id = "tax-exemption"
summary = "How tax is calculated, when exemption certificates apply, and what changes on future invoices."
canonical_questions = ["why was i charged tax", "vat invoice", "how do i add a tax exemption certificate"]
intent = "policy"
entity = "billing"
audience = "customer"
related = ["invoice-download", "refunds-policy"]
external_refs = ["https://example.com/tax"]
schema_type = "FAQPage"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "optional"
token_budget = "medium"
retrieval_aliases = ["sales tax", "vat", "tax certificate"]
+++

Tax is calculated from the sold-to address, product type, and any exemption
status on file at the time the invoice is finalized.

## Important rules

- Updated tax IDs apply only to future invoices.
- Exemption certificates are validated manually before they take effect.
- Reverse-charge treatment requires a valid business tax ID where supported.

## Submission checklist

1. Legal entity name matches the billing profile.
2. Registration number is complete and unexpired.
3. Certificate covers the sold-to jurisdiction on the invoice.

## Example review response

```text
We received your certificate and will apply it to future invoices once
validation completes. Previously finalized invoices are not reissued unless
required by local law.
```
