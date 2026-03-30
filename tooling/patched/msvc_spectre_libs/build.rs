fn main() {
    #[cfg(windows)]
    add_spectre_link_search();
}

#[cfg(windows)]
fn add_spectre_link_search() {
    use std::env;

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ENV");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=VCToolsInstallDir");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }

    let Ok(target) = env::var("TARGET") else {
        return;
    };

    let arch = match env::var("CARGO_CFG_TARGET_ARCH").ok().as_deref() {
        Some("x86_64") => "x64",
        Some("x86") => "x86",
        Some("aarch64") | Some("arm64ec") => "arm64",
        Some("arm") => "arm32",
        Some(arch) => {
            println!("cargo:warning=unsupported spectre library arch: {arch}");
            return;
        }
        None => return,
    };

    if let Some(spectre_libs) = spectre_lib_candidates(&target, arch)
        .into_iter()
        .find(|path| path.exists())
    {
        println!("cargo:rustc-link-search=native={}", spectre_libs.display());
        return;
    }

    // Zed only needs this crate to add an extra search path when Spectre libs exist.
    // Treating their absence as fatal breaks Windows builds even when the normal MSVC
    // runtime libraries are sufficient.
    println!(
        "cargo:warning=No spectre-mitigated libs were found. Continuing without adding a spectre search path."
    );
}

#[cfg(windows)]
fn spectre_lib_candidates(target: &str, arch: &str) -> Vec<std::path::PathBuf> {
    use cc::windows_registry;
    use std::{env, path::PathBuf};

    let mut candidates = Vec::new();

    if let Some(vc_tools_install_dir) = env::var_os("VCToolsInstallDir") {
        push_spectre_candidates(&mut candidates, PathBuf::from(vc_tools_install_dir), arch);
    }

    if let Some(tool) = windows_registry::find_tool(target, "cl.exe") {
        if let Some(tool_dir) = tool.path().parent() {
            push_spectre_candidates(&mut candidates, tool_dir.join(r"..\..\.."), arch);
        }
    }

    candidates
}

#[cfg(windows)]
fn push_spectre_candidates(
    candidates: &mut Vec<std::path::PathBuf>,
    root: std::path::PathBuf,
    arch: &str,
) {
    candidates.push(root.join("lib").join("spectre").join(arch));
    candidates.push(root.join("lib").join(arch).join("spectre"));
}
