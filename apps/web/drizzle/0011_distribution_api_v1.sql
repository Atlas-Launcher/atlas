CREATE TYPE "public"."distribution_product" AS ENUM('launcher', 'cli', 'runner', 'runnerd');
--> statement-breakpoint
CREATE TYPE "public"."distribution_os" AS ENUM('windows', 'macos', 'linux');
--> statement-breakpoint
CREATE TYPE "public"."distribution_arch" AS ENUM('x64', 'arm64');
--> statement-breakpoint
CREATE TYPE "public"."distribution_channel" AS ENUM('stable', 'beta', 'dev');
--> statement-breakpoint
CREATE TYPE "public"."distribution_artifact_kind" AS ENUM('installer', 'binary', 'signature', 'updater-manifest', 'other');
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "distribution_releases" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "product" "distribution_product" NOT NULL,
  "version" text NOT NULL,
  "channel" "distribution_channel" DEFAULT 'stable' NOT NULL,
  "published_at" timestamp with time zone DEFAULT now() NOT NULL,
  "notes" text DEFAULT '' NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "distribution_release_platforms" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "release_id" uuid NOT NULL,
  "os" "distribution_os" NOT NULL,
  "arch" "distribution_arch" NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "distribution_artifacts" (
  "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
  "platform_id" uuid NOT NULL,
  "kind" "distribution_artifact_kind" NOT NULL,
  "filename" text NOT NULL,
  "size" bigint NOT NULL,
  "sha256" text NOT NULL,
  "download_id" text NOT NULL,
  "artifact_ref" text NOT NULL,
  "created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "distribution_release_platforms" ADD CONSTRAINT "distribution_release_platforms_release_id_distribution_releases_id_fk" FOREIGN KEY ("release_id") REFERENCES "public"."distribution_releases"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "distribution_artifacts" ADD CONSTRAINT "distribution_artifacts_platform_id_distribution_release_platforms_id_fk" FOREIGN KEY ("platform_id") REFERENCES "public"."distribution_release_platforms"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "distribution_releases_product_version_channel_unique" ON "distribution_releases" USING btree ("product","version","channel");
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "distribution_releases_product_channel_published_at_idx" ON "distribution_releases" USING btree ("product","channel","published_at");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "distribution_release_platforms_release_os_arch_unique" ON "distribution_release_platforms" USING btree ("release_id","os","arch");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "distribution_artifacts_download_id_unique" ON "distribution_artifacts" USING btree ("download_id");
--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "distribution_artifacts_platform_filename_unique" ON "distribution_artifacts" USING btree ("platform_id","filename");
