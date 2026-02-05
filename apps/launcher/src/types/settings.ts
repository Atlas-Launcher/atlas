export type ModLoaderKind = "vanilla" | "fabric" | "neoforge";
export type InstanceSource = "local" | "atlas";
export type AtlasPackChannel = "dev" | "beta" | "production";

export interface ModLoaderConfig {
  kind: ModLoaderKind;
  loaderVersion?: string | null;
}

export interface AtlasPackLink {
  packId: string;
  packSlug: string;
  channel: AtlasPackChannel;
  buildId?: string | null;
  buildVersion?: string | null;
  artifactKey?: string | null;
}

export interface InstanceConfig {
  id: string;
  name: string;
  gameDir: string;
  version?: string | null;
  loader: ModLoaderConfig;
  javaPath?: string | null;
  memoryMb?: number | null;
  jvmArgs?: string | null;
  source?: InstanceSource;
  atlasPack?: AtlasPackLink | null;
}

export interface AppSettings {
  msClientId?: string | null;
  atlasHubUrl?: string | null;
  defaultMemoryMb?: number | null;
  defaultJvmArgs?: string | null;
  instances?: InstanceConfig[];
  selectedInstanceId?: string | null;
}
