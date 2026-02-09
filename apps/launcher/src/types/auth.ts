export interface Profile {
  id: string;
  name: string;
}

export interface AtlasProfile {
  id: string;
  email?: string | null;
  name?: string | null;
  mojang_username?: string | null;
  mojang_uuid?: string | null;
}

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
}

export type AuthFlow = "deeplink" | "device_code";

export interface LauncherLinkSession {
  linkSessionId: string;
  linkCode: string;
  proof: string;
  expiresAt: string;
}

export interface LauncherLinkComplete {
  success: boolean;
  userId: string;
  warning?: string | null;
}
