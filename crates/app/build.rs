fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon.ico");
        resource.set("FileDescription", "SQLRestore");
        resource.set("ProductName", "SQLRestore");
        resource.set("OriginalFilename", "sqlrestore.exe");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=failed to compile Windows resources: {error}");
            std::process::exit(1);
        }
    }
}
