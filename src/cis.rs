use super::common::ApiResponse;
use chrono::prelude::*;
use reqwest::{self, Method, Response, StatusCode};

// Cis module error type
#[derive(Debug, Display, From)]
pub enum Error {
    #[display(fmt = "Reqwest error: {}," _0)]
    Reqwest(reqwest::Error),
    #[display(fmt = "Unauthorized")]
    Unauthorized,
    #[display(fmt = "Unexpected status code: {}", _0)]
    UnexpectedStatusCode(u16),
}

// Represents the login status as returned from the vSphere API 
#[derive(Deserialize, Debug)]
pub struct LoginStatus {
    user: String,
    created_time: DateTime<Utc>,
    last_accessed_time: DateTime<Utc>,
}

// This type represents a vSphere Session and handles login
pub struct Session<'a> {
    hostname: &'a str,
    client: reqwest::Client,
    session_id: Option<String>,
    logged_in_user: Option<&'a str>,
}

impl<'a> Session<'a> {
    pub fn new(hostname: &'a str, insecure_certs: bool) -> Result<Self, Error> {
        let builder = reqwest::Client::builder()
            .danger_accept_invalid_certs(insecure_certs)
            .use_rustls_tls();
        Ok(Session {
            hostname,
            client: builder.build()?,
            session_id: None,
            logged_in_user: None,
        })
    }

    pub async fn login(
        &mut self,
        username: &'a str,
        password: Option<&str>,
    ) -> Result<bool, Error> {
        let resp: Response = self
            .client
            .request(
                Method::POST,
                api_url!(self.hostname, "/com/vmware/cis/session"),
            )
            .basic_auth(username, password)
            .send()
            .await?;
        let status = resp.status();
        let resp: ApiResponse<String> = match status {
            StatusCode::OK => resp.json::<ApiResponse<String>>().await?,
            StatusCode::UNAUTHORIZED => return Ok(false),
            _ => return Err(Error::UnexpectedStatusCode(status.as_u16())),
        };
        self.session_id = Some(resp.value);
        self.logged_in_user = Some(username);
        Ok(true)
    }

    fn authenticated_request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        let session_id = match self.session_id {
            Some(ref session_id) => session_id,
            None => "",
        };
        self.client
            .request(method, url)
            .header("vmware-api-session-id", session_id)
    }

    pub async fn login_status(&mut self) -> Result<LoginStatus, Error> {
        let resp: Response = self
            .authenticated_request(
                Method::POST,
                api_url!(self.hostname, "/com/vmware/cis/session?~action=get"),
            )
            .send()
            .await?;
        let status = resp.status();
        let resp: ApiResponse<LoginStatus> = match status {
            StatusCode::OK => resp.json::<ApiResponse<LoginStatus>>().await?,
            StatusCode::UNAUTHORIZED => return Err(Error::Unauthorized),
            _ => return Err(Error::UnexpectedStatusCode(status.as_u16())),
        };

        Ok(resp.value)
    }

    pub async fn logout(&mut self) -> Result<(), Error> {
        let status: StatusCode = self
            .authenticated_request(
                Method::DELETE,
                api_url!(self.hostname, "/com/vmware/cis/session"),
            )
            .send()
            .await?
            .status();
        match status {
            StatusCode::OK => {
                self.session_id = None;
                self.logged_in_user = None;
                Ok(())
            }
            StatusCode::UNAUTHORIZED => Ok(()),
            _ => Err(Error::UnexpectedStatusCode(status.as_u16())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LoginStatus, Session};

    const VCENTER_HOSTNAME: &str = "";
    const VCENTER_USERNAME: &str = "";
    const VCENTER_PASSWORD: &str = "";

    #[tokio::test]
    async fn login_login_status_logout() {
        let mut session = Session::new(VCENTER_HOSTNAME, true).expect("Session::new");
        let login_ok = session.login(VCENTER_USERNAME, Some("abc")).await.expect("session.login");
        assert!(!login_ok);
        let login_ok = session
            .login(VCENTER_USERNAME, Some(VCENTER_PASSWORD))
            .await
            .expect("login");
        assert!(login_ok);
        let login_status: LoginStatus = session.login_status().await.expect("session.login_status");
        assert_eq!(login_status.user, VCENTER_USERNAME);
        session.logout().await.expect("session.logout");
    }
}
