//! Link directives

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_DAVE_FFI");
    if std::env::var_os("CARGO_FEATURE_DAVE_FFI").is_none() {
        return; // stub build: nothing to link.
    }

    println!("cargo:rerun-if-env-changed=LIBDAVE_PREFIX");
    println!("cargo:rerun-if-env-changed=LIBDAVE_TRIPLET");

    let prefix = match std::env::var("LIBDAVE_PREFIX") {
        Ok(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => {
            let manifest = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR is set by cargo for build scripts");
            PathBuf::from(manifest).join("vendor/libdave/cpp")
        }
    };
    let prefix = std::fs::canonicalize(&prefix).unwrap_or(prefix);
    let triplet = std::env::var("LIBDAVE_TRIPLET").unwrap_or_else(|_| "x64-linux".to_string());

    let build_dir = prefix.join("build");
    let capi_dir = build_dir.join("test/capi");
    let vcpkg_lib = build_dir.join("vcpkg_installed").join(&triplet).join("lib");

    let must_exist = [
        ("external_sender.a", capi_dir.join("external_sender.a")),
        ("libdave.a", build_dir.join("libdave.a")),
        ("libmlspp.a", vcpkg_lib.join("libmlspp.a")),
        ("libcrypto.a", vcpkg_lib.join("libcrypto.a")),
    ];
    let missing: Vec<&str> = must_exist
        .iter()
        .filter(|(_, p)| !p.exists())
        .map(|(n, _)| *n)
        .collect();
    assert!(
        missing.is_empty(),
        "ug-dave: feature `dave-ffi` requires a prebuilt libdave under LIBDAVE_PREFIX={} \
         (triplet {triplet}), but these archives are missing: {missing:?}. \
         Run scripts/build_libdave.sh.",
        prefix.display()
    );

    for dir in [&build_dir, &capi_dir, &vcpkg_lib] {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!("cargo:rustc-link-lib=static:+verbatim=external_sender.a");
    for lib in ["dave", "mlspp", "hpke", "tls_syntax", "bytes", "ssl", "crypto"] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=m");

    println!("cargo:rerun-if-changed={}", build_dir.join("libdave.a").display());
    println!("cargo:rerun-if-changed={}", capi_dir.join("external_sender.a").display());
}
