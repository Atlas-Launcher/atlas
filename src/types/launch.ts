export interface LaunchEvent {
  phase: string;
  message: string;
  current?: number;
  total?: number;
  percent?: number;
}
