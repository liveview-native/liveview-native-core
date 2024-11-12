#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MergeError {
    #[error("Fragment type mismatch")]
    FragmentTypeMismatch,
    #[error("Create component from update")]
    CreateComponentFromUpdate,
    #[error("Create child from update fragment")]
    CreateChildFromUpdateFragment,
    #[error("Add child to existing")]
    AddChildToExisting,
    #[error("There was a id mismatch when merging a stream")]
    StreamIDMismatch,
    #[error("Stream Error {error}")]
    Stream {
        #[from]
        error: StreamConversionError,
    },
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum RenderError {
    #[error("No components found when needed")]
    NoComponents,
    #[error("No templates found when needed")]
    NoTemplates,
    #[error("Templated ID {0} not found in templates")]
    TemplateNotFound(i32),
    #[error("Component ID {0} not found in components")]
    ComponentNotFound(i32),
    #[error("Merge Error {0}")]
    MergeError(#[from] MergeError),
    #[error("Child {0} for template")]
    ChildNotFoundForTemplate(i32),
    #[error("Child {0} not found for static")]
    ChildNotFoundForStatic(i32),
    #[error("Cousin not found for {0}")]
    CousinNotFound(i32),
    #[error("Serde Error {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Parse Error {0}")]
    ParseError(#[from] crate::parser::ParseError),
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum StreamConversionError {
    #[error("There was no stream ID for this ")]
    NoStreamID,
}
