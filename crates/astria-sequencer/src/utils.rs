pub(crate) fn hash(s: &[u8]) -> Vec<u8> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(s);
    hasher.finalize().to_vec()
}
