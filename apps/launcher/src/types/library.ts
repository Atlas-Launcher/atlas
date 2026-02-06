export interface VersionSummary {
  id: string;
  kind: string;
}

export interface VersionManifestSummary {
  latestRelease: string;
  versions: VersionSummary[];
}

export interface FabricLoaderVersion {
  version: string;
  stable: boolean;
}

export type NeoForgeLoaderVersion = string;

export interface ModEntry {
  fileName: string;
  displayName: string;
  enabled: boolean;
  size: number;
  modified: number;
}

export type AtlasChannel = "dev" | "beta" | "production";

export interface AtlasRemotePack {
  packId: string;
  packName: string;
  packSlug: string;
  accessLevel: AtlasChannel;
  channel: AtlasChannel;
  buildId?: string | null;
  buildVersion?: string | null;
  artifactKey?: string | null;
}

export interface AtlasPackSyncResult {
  packId: string;
  channel: AtlasChannel;
  buildId?: string | null;
  buildVersion?: string | null;
  minecraftVersion?: string | null;
  modloader?: string | null;
  modloaderVersion?: string | null;
  bundledFiles: number;
  hydratedAssets: number;
}
