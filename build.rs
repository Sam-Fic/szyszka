fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let gresource_xml = std::path::Path::new("data/com.github.samfic.szyszka.gresource.xml");

    let status = std::process::Command::new("glib-compile-resources")
        .args([
            "--sourcedir", "data",
            "--target", &format!("{}/com.github.samfic.szyszka.gresource", out_dir),
            gresource_xml.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run glib-compile-resources. Install libglib2.0-dev-bin.");

    if !status.success() {
        panic!("glib-compile-resources failed");
    }

    println!("cargo:rerun-if-changed={}", gresource_xml.display());
    println!("cargo:rerun-if-changed=data/icons/com.github.samfic.szyszka.svg");
}
