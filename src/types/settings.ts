export type ModLoaderKind = "vanilla" | "fabric" | "neoforge";

export interface ModLoaderConfig {
  kind: ModLoaderKind;
  loaderVersion?: string | null;
}

export interface InstanceConfig {
  id: string;
  name: string;
  gameDir: string;
  version?: string | null;
  loader: ModLoaderConfig;
  javaPath?: string | null;
  memoryMb?: number;
}

export interface AppSettings {
  msClientId?: string | null;
  instances?: InstanceConfig[];
  selectedInstanceId?: string | null;
}
