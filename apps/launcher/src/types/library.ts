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
