#[cfg(target_os = "macos")]
uniffi::build_foreign_language_testcases!(
    "tests/bindings/simple.kts",
    "tests/bindings/simple.swift",
);
