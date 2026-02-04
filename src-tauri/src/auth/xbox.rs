use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::http::post_json;

const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

#[derive(Debug, Deserialize)]
pub struct XboxAuthResponse {
  pub Token: String,
  pub DisplayClaims: XboxDisplayClaims
}

#[derive(Debug, Deserialize)]
pub struct XboxDisplayClaims {
  pub xui: Vec<XboxUserClaim>
}

#[derive(Debug, Deserialize)]
pub struct XboxUserClaim {
  pub uhs: String
}

pub async fn authenticate(ms_access_token: &str) -> Result<XboxAuthResponse, String> {
  let client = Client::new();
  let body = json!({
    "Properties": {
      "AuthMethod": "RPS",
      "SiteName": "user.auth.xboxlive.com",
      "RpsTicket": format!("d={}", ms_access_token)
    },
    "RelyingParty": "http://auth.xboxlive.com",
    "TokenType": "JWT"
  });

  post_json(&client, XBL_AUTH_URL, &body).await
}

pub async fn xsts(xbl_token: &str) -> Result<XboxAuthResponse, String> {
  let client = Client::new();
  let body = json!({
    "Properties": {
      "SandboxId": "RETAIL",
      "UserTokens": [xbl_token]
    },
    "RelyingParty": "rp://api.minecraftservices.com/",
    "TokenType": "JWT"
  });

  post_json(&client, XSTS_AUTH_URL, &body).await
}
