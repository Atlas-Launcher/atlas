import { S3Client, PutObjectCommand, GetObjectCommand } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

function getConfig() {
  const accountId = process.env.R2_ACCOUNT_ID;
  const accessKeyId = process.env.R2_ACCESS_KEY_ID;
  const secretAccessKey = process.env.R2_SECRET_ACCESS_KEY;
  const bucket = process.env.R2_BUCKET;

  if (!accountId || !accessKeyId || !secretAccessKey || !bucket) {
    throw new Error("R2 environment variables are not fully configured");
  }

  return {
    bucket,
    endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
    credentials: {
      accessKeyId,
      secretAccessKey,
    },
  };
}

export function isR2Configured() {
  return Boolean(
    process.env.R2_ACCOUNT_ID &&
      process.env.R2_ACCESS_KEY_ID &&
      process.env.R2_SECRET_ACCESS_KEY &&
      process.env.R2_BUCKET
  );
}

function createClient() {
  const config = getConfig();
  return new S3Client({
    region: "auto",
    endpoint: config.endpoint,
    credentials: config.credentials,
  });
}

export function getBucketName() {
  return getConfig().bucket;
}

export async function createPresignedUploadUrl({
  key,
  contentType,
  expiresIn = 900,
}: {
  key: string;
  contentType?: string;
  expiresIn?: number;
}) {
  const config = getConfig();
  const client = createClient();
  const command = new PutObjectCommand({
    Bucket: config.bucket,
    Key: key,
    ContentType: contentType,
  });

  return getSignedUrl(client, command, { expiresIn });
}

export async function createPresignedDownloadUrl({
  key,
  expiresIn = 900,
}: {
  key: string;
  expiresIn?: number;
}) {
  const config = getConfig();
  const client = createClient();
  const command = new GetObjectCommand({
    Bucket: config.bucket,
    Key: key,
  });

  return getSignedUrl(client, command, { expiresIn });
}

export async function uploadToR2({
  key,
  body,
  contentType,
}: {
  key: string;
  body: ArrayBuffer;
  contentType?: string;
}) {
  const config = getConfig();
  const client = createClient();
  const command = new PutObjectCommand({
    Bucket: config.bucket,
    Key: key,
    Body: Buffer.from(body),
    ContentType: contentType,
  });
  await client.send(command);
}

export async function downloadFromR2(key: string) {
  const config = getConfig();
  const client = createClient();
  const command = new GetObjectCommand({
    Bucket: config.bucket,
    Key: key,
  });
  const output = await client.send(command);
  if (!output.Body) {
    throw new Error("R2 returned an empty response body.");
  }

  const stream = output.Body.transformToWebStream();
  return {
    stream,
    contentType: output.ContentType ?? "application/octet-stream",
    contentLength: output.ContentLength ?? undefined,
  };
}
