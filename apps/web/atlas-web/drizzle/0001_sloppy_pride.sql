ALTER TABLE "builds" DROP CONSTRAINT IF EXISTS "builds_pack_id_packs_id_fk";--> statement-breakpoint
ALTER TABLE "channels" DROP CONSTRAINT IF EXISTS "channels_pack_id_packs_id_fk";--> statement-breakpoint
ALTER TABLE "invites" DROP CONSTRAINT IF EXISTS "invites_pack_id_packs_id_fk";--> statement-breakpoint
ALTER TABLE "pack_members" DROP CONSTRAINT IF EXISTS "pack_members_pack_id_packs_id_fk";--> statement-breakpoint
ALTER TABLE "builds" ALTER COLUMN "pack_id" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "channels" ALTER COLUMN "pack_id" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "invites" ALTER COLUMN "pack_id" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "pack_members" ALTER COLUMN "pack_id" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "packs" ALTER COLUMN "id" SET DATA TYPE text;--> statement-breakpoint
ALTER TABLE "packs" ALTER COLUMN "id" DROP DEFAULT;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "builds" ADD CONSTRAINT "builds_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "channels" ADD CONSTRAINT "channels_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invites" ADD CONSTRAINT "invites_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "pack_members" ADD CONSTRAINT "pack_members_pack_id_packs_id_fk" FOREIGN KEY ("pack_id") REFERENCES "public"."packs"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
