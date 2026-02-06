ALTER TABLE "builds"
ADD COLUMN "force_reinstall" boolean NOT NULL DEFAULT false;
