ALTER TABLE public.payments
  DROP COLUMN IF EXISTS currency;

ALTER TABLE public.invoices
  DROP COLUMN IF EXISTS currency;
