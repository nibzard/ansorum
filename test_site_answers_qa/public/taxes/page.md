---
id: tax-exemption
canonical_url: https://answers.example.com/taxes/
visibility: public
ai_visibility: public
intent: policy
entity: billing
audience: customer
llms_priority: optional
token_budget: medium
canonical_questions:
- why was i charged tax
- vat invoice
- how do i add a tax exemption certificate
retrieval_aliases:
- sales tax
- vat
- tax certificate
related:
- invoice-download
- refunds-policy
---

# Sales tax, VAT, and tax exemption

How tax is calculated, when exemption certificates apply, and what changes on future invoices.

Tax is calculated from the sold-to address, product type, and any exemption
status on file at the time the invoice is finalized.

## Important rules

- Updated tax IDs apply only to future invoices.
- Exemption certificates are validated manually before they take effect.
- Reverse-charge treatment requires a valid business tax ID where supported.

## Submission checklist

- Legal entity name matches the billing profile.
- Registration number is complete and unexpired.
- Certificate covers the sold-to jurisdiction on the invoice.

## Example review response

We received your certificate and will apply it to future invoices once
validation completes. Previously finalized invoices are not reissued unless
required by local law.
