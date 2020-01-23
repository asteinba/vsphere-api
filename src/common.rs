// Generate a URL for the vSphere API of the given hostname
macro_rules! api_url {
    ($hostname:expr, $endpoint:expr) => {
        &format!("https://{}/rest/{}", $hostname, $endpoint)
    };
}

// Generic value container which is widely used in the vSphere API
#[derive(Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub value: T,
}
