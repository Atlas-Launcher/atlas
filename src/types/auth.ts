export interface Profile {
  id: string;
  name: string;
}

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
}

export type AuthFlow = "deeplink" | "device_code";
