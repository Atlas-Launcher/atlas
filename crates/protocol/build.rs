fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/atlas.proto");

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    let mut config = prost_build::Config::new();
    config.btree_map(&[".atlas.protocol.PackBlob.files"]);
    config.compile_protos(&["proto/atlas.proto"], &["proto"])?;

    Ok(())
}
