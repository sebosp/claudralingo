//! The Windows executable gets its icon and version resource (`packaging/windows/claudralingo.ico`,
//! the fields of Cargo.toml). The other platforms have no build step.

fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/claudralingo.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/windows/claudralingo.ico");
        res.set("FileDescription", "Claudralingo typing tutor");
        res.set("LegalCopyright", "GPL-3.0-or-later");
        res.compile().expect("the Windows resource file");
    }
}
