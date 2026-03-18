+++
title = "Download invoices and receipts"
weight = 30

id = "invoice-download"
summary = "Where invoices live, who can export them, and how receipts differ from invoice PDFs."
canonical_questions = ["where can i find invoices", "download receipt", "export billing history"]
intent = "task"
entity = "billing"
audience = "customer"
related = ["cancel-subscription", "tax-exemption", "payment-recovery"]
external_refs = ["https://example.com/invoices"]
schema_type = "HowTo"
review_by = 2026-09-01
visibility = "public"
ai_visibility = "public"
llms_priority = "core"
token_budget = "medium"
retrieval_aliases = ["invoice pdf", "receipt download", "billing history export"]
+++

Owners and billing admins can export invoice PDFs, card receipts, and a CSV
ledger of paid invoices from the billing portal.

## Document types

| Document | Generated when | Includes |
| --- | --- | --- |
| Invoice PDF | An invoice is finalized | Billing address, tax lines, payment terms |
| Card receipt | A card payment settles | Authorization reference and payment date |
| Ledger CSV | Export requested | Invoice IDs, totals, tax, and payment status |

## Export from the UI

1. Go to **Settings -> Billing -> Invoices**.
2. Filter by status, date range, or workspace.
3. Open an invoice row and select **Download PDF** or **Download receipt**.

## Export via API

```bash
curl -H "Authorization: Bearer $API_TOKEN" \
  "https://api.example.com/v1/billing/invoices?workspace=acme&status=paid"
```

Use the API when finance needs a repeatable month-end export or when multiple
child workspaces roll up into one ledger.

## Troubleshooting

- Missing receipt: card settlement may still be pending.
- Missing invoice: check whether the charge belongs to a different workspace.
- Wrong address on invoice: update billing details before the next invoice finalizes.
