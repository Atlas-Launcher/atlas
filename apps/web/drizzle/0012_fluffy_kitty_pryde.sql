CREATE TABLE IF NOT EXISTS "app_deploy_tokens" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"name" text,
	"token_hash" text NOT NULL,
	"token_prefix" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_used_at" timestamp with time zone,
	"revoked_at" timestamp with time zone,
	"expires_at" timestamp with time zone
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "pack_deploy_tokens" (
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
 ALTER TABLE "pack_deploy_tokens" ADD CONSTRAINT "pack_deploy_tokens_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "app_deploy_tokens_prefix_idx" ON "app_deploy_tokens" USING btree ("token_prefix");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "pack_deploy_tokens_pack_idx" ON "pack_deploy_tokens" USING btree ("pack_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "pack_deploy_tokens_prefix_idx" ON "pack_deploy_tokens" USING btree ("token_prefix");
