export interface ReadinessItem {
  key: string;
  label: string;
  ready: boolean;
  detail?: string | null;
}

export interface LaunchReadinessReport {
  atlasLoggedIn: boolean;
  microsoftLoggedIn: boolean;
  accountsLinked: boolean;
  filesInstalled: boolean;
  javaReady: boolean;
  readyToLaunch: boolean;
  checklist: ReadinessItem[];
}

export type FixAction =
  | "relinkAccount"
  | "setSafeMemory"
  | "resyncPack"
  | "repairRuntime"
  | "fullRepair";

export interface TroubleshooterFinding {
  code: string;
  title: string;
  detail: string;
  confidence: number;
  suggestedActions: FixAction[];
}

export interface TroubleshooterReport {
  readiness: LaunchReadinessReport;
  findings: TroubleshooterFinding[];
}

export interface FixResult {
  action: FixAction;
  applied: boolean;
  message: string;
}

export interface RepairResult {
  repaired: boolean;
  message: string;
  details: string[];
}
