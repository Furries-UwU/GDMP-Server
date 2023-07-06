use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/packet.proto"], &["proto/"])?;
    Ok(())
}
