fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("band.ico");
        resource.set("ProductName", "Anyinstaller Packager GUI");
        resource.set("FileDescription", "Anyinstaller Packager GUI");
        resource
            .compile()
            .expect("failed to embed Windows resources");
    }
}
