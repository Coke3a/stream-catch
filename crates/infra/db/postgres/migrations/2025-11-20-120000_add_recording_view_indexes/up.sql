CREATE INDEX "recordings_status_started_at_id_idx"
  ON "recordings" ("status", "started_at" DESC, "id" DESC);

CREATE INDEX "follows_user_status_live_account_idx"
  ON "follows" ("user_id", "status", "live_account_id");
