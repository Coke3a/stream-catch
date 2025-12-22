ALTER TABLE public.invoices
  ADD COLUMN currency TEXT NOT NULL DEFAULT 'usd';

ALTER TABLE public.payments
  ADD COLUMN currency TEXT NOT NULL DEFAULT 'usd';
