fn main() {
    #[cfg(feature = "ndi-sdk")]
    {
        #[cfg(target_os = "macos")]
        {
            let ndi_path = "/Library/NDI SDK for Apple/lib/macOS";
            println!("cargo:rustc-link-lib=dylib=ndi");
            println!("cargo:rustc-link-search=native={}", ndi_path);
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", ndi_path);
            println!("cargo:rerun-if-changed={}/libndi.dylib", ndi_path);
        }

        #[cfg(target_os = "windows")]
        {
            println!("cargo:rustc-link-lib=Processing.NDI.Lib.x64");
            println!("cargo:rustc-link-search=native=C:/Program Files/NDI/NDI 5 SDK/Lib/x64");
        }

        #[cfg(target_os = "linux")]
        {
            println!("cargo:rustc-link-lib=ndi");
            println!("cargo:rustc-link-search=native=/usr/local/lib");
        }
    }
}
