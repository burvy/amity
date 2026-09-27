// use powershell:
// Install-Module -Name Posh-ACME -Scope CurrentUser
// Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
// Import-Module Posh-ACME
// Set-PAServer LE_PROD
// New-PACertificate '192.0.2.1(replace with your public ip)' `
//   -Plugin WebRoot `
//   -PluginArgs @{ WRPath = 'C:\acme-challenge' } `
//   -Profile shortlived `
//   -AcceptTOS `
//   -Contact 'name@email.com(replace with your email)'
#[cfg(not(feature = "dev-local"))]
pub const KEY_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\cert.key";
#[cfg(not(feature = "dev-local"))]
pub const CERT_PATH: &str =
    r"C:\Users\Burvy\AppData\Local\Posh-ACME\LE_PROD\3716529536\174.175.161.63\fullchain.cer";

/// Seconds without packets before a client is dropped. Under manual
/// authentication this is inert
pub const CLIENT_TIMEOUT_SECS: i32 = 300;

/// Seconds between heartbeat log lines
pub const HEARTBEAT_SECS: f32 = 60.0;
