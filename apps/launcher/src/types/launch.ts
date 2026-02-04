import type { ModLoaderConfig } from "./settings";

export interface LaunchEvent {
  phase: string;
  message: string;
  current?: number;
  total?: number;
  percent?: number;
}

export interface LaunchOptions {
  gameDir: string;
  javaPath?: string;
  memoryMb?: number;
  version?: string | null;
  loader?: ModLoaderConfig;
}
