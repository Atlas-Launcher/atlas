CREATE TABLE "launcher_link_sessions" (
  "id" text PRIMARY KEY NOT NULL,
  "code" text NOT NULL,
  "proof" text NOT NULL,
  "expires_at" timestamp with time zone NOT NULL,
  "claimed_user_id" text,
  "claimed_at" timestamp with time zone,
  "completed_at" timestamp with time zone,
  "minecraft_uuid" text,
  "minecraft_name" text,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX "launcher_link_sessions_code_unique" ON "launcher_link_sessions" ("code");
--> statement-breakpoint
CREATE INDEX "launcher_link_sessions_claimed_user_idx" ON "launcher_link_sessions" ("claimed_user_id");
--> statement-breakpoint
CREATE INDEX "launcher_link_sessions_minecraft_uuid_idx" ON "launcher_link_sessions" ("minecraft_uuid");
--> statement-breakpoint
ALTER TABLE "launcher_link_sessions" ADD CONSTRAINT "launcher_link_sessions_claimed_user_id_user_id_fk" FOREIGN KEY ("claimed_user_id") REFERENCES "public"."user"("id") ON DELETE set null ON UPDATE no action;
--> statement-breakpoint
CREATE UNIQUE INDEX "user_mojang_uuid_unique" ON "user" ("mojang_uuid");
