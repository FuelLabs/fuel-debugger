//! Dumps GraphQL schema from fuel_core automatically

use fuel_core::schema::build_schema;
use std::{env, fs::File, io::Write, path::PathBuf};


fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");

    let target_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .as_path()
        .join("schema.graphql");

    File::create(&target_path)
        .and_then(|mut f| {
            f.write_all(build_schema().finish().sdl().as_bytes())?;
            f.sync_all()
        })
        .expect("Failed to write SDL schema file");
}
