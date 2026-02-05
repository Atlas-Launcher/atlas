use serde::Deserialize;
use serde_json::json;

use super::error::AuthError;
use crate::net::http::HttpClient;

const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

#[derive(Debug, Deserialize)]
pub struct XboxAuthResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
pub struct XboxDisplayClaims {
    pub xui: Vec<XboxUserClaim>,
}

#[derive(Debug, Deserialize)]
pub struct XboxUserClaim {
    pub uhs: String,
}

pub async fn authenticate<H: HttpClient + ?Sized>(
    http: &H,
    ms_access_token: &str,
) -> Result<XboxAuthResponse, AuthError> {
    let body = json!({
      "Properties": {
        "AuthMethod": "RPS",
        "SiteName": "user.auth.xboxlive.com",
        "RpsTicket": format!("d={}", ms_access_token)
      },
      "RelyingParty": "http://auth.xboxlive.com",
      "TokenType": "JWT"
    });

    Ok(http.post_json(XBL_AUTH_URL, &body).await?)
}

pub async fn xsts<H: HttpClient + ?Sized>(
    http: &H,
    xbl_token: &str,
) -> Result<XboxAuthResponse, AuthError> {
    let body = json!({
      "Properties": {
        "SandboxId": "RETAIL",
        "UserTokens": [xbl_token]
      },
      "RelyingParty": "rp://api.minecraftservices.com/",
      "TokenType": "JWT"
    });

    Ok(http.post_json(XSTS_AUTH_URL, &body).await?)
}
