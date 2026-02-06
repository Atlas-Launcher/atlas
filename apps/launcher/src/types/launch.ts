import type { ModLoaderConfig } from "./settings";

export interface LaunchEvent {
  phase: string;
  message: string;
  current?: number;
  total?: number;
  percent?: number;
}

export interface LaunchLogEvent {
  stream: string;
  message: string;
}

export interface LaunchOptions {
  gameDir: string;
  javaPath?: string;
  memoryMb?: number;
  jvmArgs?: string;
  version?: string | null;
  loader?: ModLoaderConfig;
}
