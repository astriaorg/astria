fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("bridge-signer-v")?;
    Ok(())
}
