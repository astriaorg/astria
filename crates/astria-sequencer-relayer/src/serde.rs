//! Utilities for serializing and deserializing bytes
use base64_serde::base64_serde_type;

base64_serde_type!(pub(crate) Base64Standard, base64::engine::general_purpose::STANDARD);
