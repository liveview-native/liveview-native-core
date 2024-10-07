#[cfg(any(target_os = "macos", target_os = "linux"))]
uniffi::build_foreign_language_testcases!(
    "tests/bindings/simple.kts",
    "tests/bindings/simple.swift",
);
