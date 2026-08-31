fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/tracks.proto")?;
    tonic_prost_build::compile_protos("proto/categories.proto")?;
    tonic_prost_build::compile_protos("proto/likes.proto")?;
    Ok(())
}