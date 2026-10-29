fn main() {
    slint_build::compile("ui/app-window.slint").expect("Unable to compile the PassGen interface");
    println!("cargo:rerun-if-changed=assets/passgen.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    use std::{env, fs, path::PathBuf, process::Command};

    let host = env::var("HOST").expect("Missing build host");
    let sdk = find_msvc_tools::find_windows_sdk(host.split('-').next().unwrap_or("x86_64"))
        .expect("Windows SDK is required to embed the application icon");
    let compiler = sdk
        .path()
        .map(|directory| directory.join("rc.exe"))
        .find(|path| path.is_file())
        .expect("Windows SDK resource compiler was not found");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Missing build output directory"));
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Missing project directory"))
        .join("assets/passgen.ico");
    let source = output.join("passgen.rc");
    let resource = output.join("passgen.res");
    fs::write(
        &source,
        format!(
            "#pragma code_page(65001)\n1 ICON \"{}\"\n",
            icon.to_string_lossy().replace('\\', "\\\\")
        ),
    )
    .expect("Unable to write the application icon resource");
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&source)
        .status()
        .expect("Unable to run the Windows resource compiler");
    assert!(status.success(), "Unable to compile the application icon");
    println!("cargo:rustc-link-arg-bin=PassGen={}", resource.display());
}
