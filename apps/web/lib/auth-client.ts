import { createAuthClient } from "better-auth/react";
import { passkeyClient } from "@better-auth/passkey/client";
import {
  deviceAuthorizationClient,
  inferAdditionalFields,
  oidcClient,
} from "better-auth/client/plugins";
import { apiKeyClient } from "better-auth/client/plugins";

export const authClient = createAuthClient({
  baseURL: process.env.NEXT_PUBLIC_BETTER_AUTH_URL ?? "",
  plugins: [
    passkeyClient(),
    deviceAuthorizationClient(),
    apiKeyClient(),
    oidcClient(),
    inferAdditionalFields({
      user: {
        role: { type: "string" },
      },
    }),
  ],
});
