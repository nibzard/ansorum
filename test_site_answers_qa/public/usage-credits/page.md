---
id: usage-credits
canonical_url: https://answers.example.com/usage-credits/
visibility: public
ai_visibility: public
intent: concept
entity: billing
audience: customer
llms_priority: optional
token_budget: medium
canonical_questions:
- how do credits work
- promo balance
- why were credits not applied
retrieval_aliases:
- credit balance
- promotional credits
- prepaid credits
related:
- invoice-download
- refunds-policy
- payment-recovery
---

# Usage credits and promotional balances

How prepaid and promotional credits are applied, how they expire, and when they do not reduce invoices.

Credits reduce eligible usage charges before the remaining balance is invoiced.
They do not automatically offset taxes, prior-due invoices, or reseller-managed
contracts.

## Credit types

- Prepaid credits purchased through sales.
- Service credits granted for a documented outage.
- Promotional credits issued for trials or launches.

## Rules that matter in QA

- Promotional credits can expire.
- Credits are applied oldest-expiring first.
- Credits usually do not cross workspace boundaries unless contractually linked.

## Example balance payload

{
  &quot;workspace&quot;: &quot;acme&quot;,
  &quot;available_cents&quot;: 125000,
  &quot;currency&quot;: &quot;USD&quot;,
  &quot;expiring&quot;: [
    { &quot;amount_cents&quot;: 25000, &quot;expires_at&quot;: &quot;2026-09-30&quot; }
  ]
}
