//! Native Mac ABI boundary. Session/control operations are added after their DTOs are frozen.
#![deny(unsafe_op_in_unsafe_fn)]

/// Version of the native Mac C ABI, independent of the application SemVer.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_abi_version() -> u32 {
    1
}

/// NUL-terminated actual Rust package version. The immutable bytes live for the process lifetime.
/// Callers must not free or mutate this static string.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_kernel_release_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// Actual shared-kernel function descriptor version, or zero if the query fails.
/// This is a library linking probe; it does not create a document/session or claim a renderer is ready.
#[unsafe(no_mangle)]
pub extern "C" fn om_host_metadata_version() -> u32 {
    std::panic::catch_unwind(|| {
        om_kernel::capabilities::capabilities(om_kernel::protocol::HostPlatform::Desktop)
            .metadata_version
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linking_probe_reports_the_real_desktop_kernel_catalog() {
        assert_eq!(om_host_abi_version(), 1);
        let catalog = om_kernel::capabilities::function_catalog();
        assert!(!catalog.functions.is_empty());
        assert_eq!(om_host_metadata_version(), catalog.metadata_version);
        let desktop =
            om_kernel::capabilities::capabilities(om_kernel::protocol::HostPlatform::Desktop);
        assert!(desktop.scene_3d);
        assert!(desktop.task_permissions.is_none());
    }
}
