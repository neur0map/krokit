use shai_core::tools::mcp::mcp_oauth::signin_oauth;

#[tokio::main]
async fn main() {
    println!("🚀 Starting OAuth flow test...");
    
    match signin_oauth("https://mcp.eu.ovhcloud.com/").await {
        Ok(access_token) => {
            println!("✅ OAuth flow completed successfully!");
            println!("🎫 Access Token: {}", access_token);
            println!("🔑 Token length: {} characters", access_token.len());
        }
        Err(e) => {
            println!("❌ OAuth flow failed: {}", e);
            std::process::exit(1);
        }
    }
}