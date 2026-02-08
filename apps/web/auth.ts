import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { apiKey, deviceAuthorization, jwt, oidcProvider } from "better-auth/plugins";
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
const launcherClientId = process.env.ATLAS_OIDC_LAUNCHER_CLIENT_ID ?? "atlas-launcher";
const launcherRedirectUrlsRaw = (
  process.env.ATLAS_OIDC_LAUNCHER_REDIRECT_URIS ?? "atlas://signin"
)
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);
const launcherRedirectUrls =
  launcherRedirectUrlsRaw.length > 0 ? launcherRedirectUrlsRaw : ["atlas://signin"];
const launcherTrustedClients =
  launcherRedirectUrls.length > 0
    ? [
      {
        clientId: launcherClientId,
        name: "Atlas Launcher",
        // Launcher uses PKCE without a client secret, so it must be a public client.
        type: "public" as const,
        disabled: false,
        redirectUrls: launcherRedirectUrls,
        skipConsent: true,
        metadata: {
          app: "atlas-launcher",
        },
      },
    ]
    : [];

export const auth = betterAuth({
  baseURL: baseUrl,
  secret: process.env.BETTER_AUTH_SECRET ?? process.env.AUTH_SECRET,
  socialProviders: {
    github: {
      clientId: process.env.GITHUB_CLIENT_ID ?? "",
      clientSecret: process.env.GITHUB_CLIENT_SECRET ?? "",
    },
    microsoft: {
      clientId: process.env.MICROSOFT_CLIENT_ID ?? "",
      clientSecret: process.env.MICROSOFT_CLIENT_SECRET ?? "",
      tenantId: "consumers",
      scope: ["openid", "profile", "email", "XboxLive.signin", "offline_access"],
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
      oauthApplication: schema.oauthApplications,
      oauthAccessToken: schema.oauthAccessTokens,
      oauthConsent: schema.oauthConsents,
      jwks: schema.jwks,
    },
  }),
  emailAndPassword: {
    enabled: true,
  },
  account: {
    accountLinking: {
      trustedProviders: ["github", "microsoft"],
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
      mojangUsername: {
        type: "string",
        required: false,
        input: false,
      },
      mojangUuid: {
        type: "string",
        required: false,
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
    jwt(),
    oidcProvider({
      loginPage: "/sign-in",
      consentPage: "/consent",
      requirePKCE: true,
      useJWTPlugin: true,
      trustedClients: launcherTrustedClients,
    }),
    deviceAuthorization({
      verificationUri: "/cli/signin",
      expiresIn: "10m",
      validateClient: async (clientId) =>
        deviceClientIds.length === 0 || deviceClientIds.includes(clientId),
    }),
  ],
});

export type Session = typeof auth.$Infer.Session;
