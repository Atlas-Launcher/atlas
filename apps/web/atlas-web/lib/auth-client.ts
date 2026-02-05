import { createAuthClient } from "better-auth/react";
import { passkeyClient } from "@better-auth/passkey/client";
import {
  deviceAuthorizationClient,
  inferAdditionalFields,
} from "better-auth/client/plugins";

export const authClient = createAuthClient({
  baseURL: process.env.NEXT_PUBLIC_BETTER_AUTH_URL ?? "",
  plugins: [
    passkeyClient(),
    deviceAuthorizationClient(),
    inferAdditionalFields({
      user: {
        role: { type: "string" },
      },
    }),
  ],
});
