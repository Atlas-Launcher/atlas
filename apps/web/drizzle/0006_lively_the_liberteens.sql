ALTER TABLE "builds" ADD COLUMN "force_reinstall" boolean DEFAULT false NOT NULL;--> statement-breakpoint
ALTER TABLE "user" ADD COLUMN "mojang_username" text;--> statement-breakpoint
ALTER TABLE "user" ADD COLUMN "mojang_uuid" text;