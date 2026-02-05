import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { deviceAuthorization } from "better-auth/plugins";
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
  database: drizzleAdapter(db, {
    provider: "pg",
    schema: {
      user: schema.users,
      session: schema.sessions,
      account: schema.accounts,
      verification: schema.verifications,
    },
  }),
  emailAndPassword: {
    enabled: true,
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
