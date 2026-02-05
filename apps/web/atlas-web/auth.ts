import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { apiKey, deviceAuthorization } from "better-auth/plugins";
import { passkey } from "@better-auth/passkey";

import { db } from "@/lib/db";
import * as schema from "@/lib/db/schema";

const baseUrl =
  process.env.BETTER_AUTH_URL ??
  process.env.NEXT_PUBLIC_BETTER_AUTH_URL ??
  "http://localhost:3000";

const url = new URL(baseUrl);
const rpID = process.env.PASSKEY_RP_ID ?? url.hostname;
const rpName = process.env.PASSKEY_RP_NAME ?? "Atlas Hub";
const passkeyOrigin = process.env.PASSKEY_ORIGIN ?? baseUrl;
const trustedOrigins = [
  baseUrl,
  process.env.ATLAS_SCHEME_ORIGIN ?? "atlas://",
].filter(Boolean);
const deviceClientIds = (process.env.ATLAS_DEVICE_CLIENT_ID ?? "atlas-launcher")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);

export const auth = betterAuth({
  baseURL: baseUrl,
  secret: process.env.BETTER_AUTH_SECRET ?? process.env.AUTH_SECRET,
  socialProviders: {
    github: {
      clientId: process.env.GITHUB_CLIENT_ID ?? "",
      clientSecret: process.env.GITHUB_CLIENT_SECRET ?? "",
      scope: ["repo", "read:org", "user:email"],
    },
  },
  database: drizzleAdapter(db, {
    provider: "pg",
    schema: {
      user: schema.users,
      session: schema.sessions,
      account: schema.accounts,
      verification: schema.verifications,
      passkey: schema.passkeys,
      deviceCode: schema.deviceCodes,
      apikey: schema.apiKeys,
    },
  }),
  emailAndPassword: {
    enabled: true,
  },
  account: {
    accountLinking: {
      trustedProviders: ["github"],
      allowDifferentEmails: true,
      updateUserInfoOnLink: true,
    },
  },
  user: {
    additionalFields: {
      role: {
        type: ["player", "creator", "admin"],
        required: false,
        defaultValue: "player",
        input: false,
      },
    },
  },
  trustedOrigins,
  plugins: [
    apiKey({
      enableMetadata: true,
      requireName: true,
      defaultPrefix: "atlas_",
      rateLimit: {
        enabled: false,
      },
    }),
    passkey({
      rpID,
      rpName,
      origin: passkeyOrigin,
    }),
    deviceAuthorization({
      verificationUri: "/device",
      expiresIn: "10m",
      validateClient: async (clientId) =>
        deviceClientIds.length === 0 || deviceClientIds.includes(clientId),
    }),
  ],
});

export type Session = typeof auth.$Infer.Session;
