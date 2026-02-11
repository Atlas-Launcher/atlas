use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessItem {
    pub key: String,
    pub label: String,
    pub ready: bool,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaunchReadinessReport {
    pub atlas_logged_in: bool,
    pub microsoft_logged_in: bool,
    pub accounts_linked: bool,
    pub files_installed: bool,
    pub java_ready: bool,
    pub ready_to_launch: bool,
    pub checklist: Vec<ReadinessItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FixAction {
    RelinkAccount,
    SetSafeMemory,
    ResyncPack,
    RepairRuntime,
    FullRepair,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TroubleshooterFinding {
    pub code: String,
    pub title: String,
    pub detail: String,
    pub confidence: u8,
    pub suggested_actions: Vec<FixAction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TroubleshooterReport {
    pub readiness: LaunchReadinessReport,
    pub findings: Vec<TroubleshooterFinding>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FixResult {
    pub action: FixAction,
    pub applied: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    pub repaired: bool,
    pub message: String,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundleResult {
    pub bundle_dir: String,
    pub report_json_path: String,
    pub summary_path: String,
    pub summary: String,
}
