---
id: invoice-download
canonical_url: https://answers.example.com/invoices/
visibility: public
ai_visibility: public
intent: task
entity: billing
audience: customer
llms_priority: core
token_budget: medium
canonical_questions:
- where can i find invoices
- download receipt
- export billing history
retrieval_aliases:
- invoice pdf
- receipt download
- billing history export
related:
- cancel-subscription
- tax-exemption
- payment-recovery
---

# Download invoices and receipts

Where invoices live, who can export them, and how receipts differ from invoice PDFs.

Owners and billing admins can export invoice PDFs, card receipts, and a CSV
ledger of paid invoices from the billing portal.

## Document types

DocumentGenerated whenIncludes
Invoice PDFAn invoice is finalizedBilling address, tax lines, payment terms
Card receiptA card payment settlesAuthorization reference and payment date
Ledger CSVExport requestedInvoice IDs, totals, tax, and payment status

## Export from the UI

- Go to **Settings -> Billing -> Invoices**.
- Filter by status, date range, or workspace.
- Open an invoice row and select **Download PDF** or **Download receipt**.

## Export via API

curl -H &quot;Authorization: Bearer $API_TOKEN&quot; \
  &quot;https://api.example.com/v1/billing/invoices?workspace=acme&amp;status=paid&quot;

Use the API when finance needs a repeatable month-end export or when multiple
child workspaces roll up into one ledger.

## Troubleshooting

- Missing receipt: card settlement may still be pending.
- Missing invoice: check whether the charge belongs to a different workspace.
- Wrong address on invoice: update billing details before the next invoice finalizes.
