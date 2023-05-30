use base64::engine::general_purpose::STANDARD;
use base64_serde::base64_serde_type;

base64_serde_type!(pub(crate) Base64Standard, STANDARD);
