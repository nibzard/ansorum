---
id: payment-recovery
canonical_url: https://answers.example.com/payment-failures/
visibility: public
ai_visibility: public
intent: reference
entity: billing
audience: customer
llms_priority: core
token_budget: medium
canonical_questions:
- payment failed
- card declined renewal
- how do retries work
retrieval_aliases:
- failed payment
- retry schedule
- declined charge
related:
- invoice-download
- cancel-subscription
- team-billing-admin
---

# What happens when a payment fails?

Retry timing, grace periods, and how to recover a failed subscription payment.

If a renewal payment fails, the invoice remains open and the system retries the
charge automatically before service is restricted.

## Retry schedule

AttemptWhenCustomer-facing effect
1Renewal timeInvoice opens as past_due
224 hours laterReminder email to billing admins
372 hours laterIn-app banner for owners
47 days laterService may enter read-only mode

## Recommended recovery steps

- Update the payment method from the billing portal.
- Reopen the past-due invoice and click **Retry payment**.
- Confirm the status changes to `paid` before closing the incident.

## Common causes

- Card expired or replaced after fraud reissue.
- Bank blocked the charge because of region or MCC restrictions.
- Invoice exceeded the card's transaction limit.

## If the workspace is already restricted

Support can temporarily extend the grace window for enterprise customers, but
that requires an open invoice ID and a documented owner request.
