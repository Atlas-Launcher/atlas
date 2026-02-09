CREATE TABLE IF NOT EXISTS "launcher_link_sessions" (
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
CREATE TABLE IF NOT EXISTS "runner_service_tokens" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"pack_id" text NOT NULL,
	"name" text,
	"token_hash" text NOT NULL,
	"token_prefix" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	"expires_at" timestamp with time zone
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "launcher_link_sessions" ADD CONSTRAINT "launcher_link_sessions_claimed_user_id_user_id_fk" FOREIGN KEY ("claimed_user_id") REFERENCES "public"."user"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "runner_service_tokens" ADD CONSTRAINT "runner_service_tokens_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "launcher_link_sessions_code_unique" ON "launcher_link_sessions" USING btree ("code");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "launcher_link_sessions_claimed_user_idx" ON "launcher_link_sessions" USING btree ("claimed_user_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "launcher_link_sessions_minecraft_uuid_idx" ON "launcher_link_sessions" USING btree ("minecraft_uuid");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "runner_service_tokens_pack_idx" ON "runner_service_tokens" USING btree ("pack_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "runner_service_tokens_prefix_idx" ON "runner_service_tokens" USING btree ("token_prefix");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "user_mojang_uuid_unique" ON "user" USING btree ("mojang_uuid");