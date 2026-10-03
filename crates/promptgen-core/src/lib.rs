#![forbid(unsafe_code)]

pub mod catalog;
mod common;
pub mod diagnostic;
pub mod image;
pub mod image_artifact;
pub mod interview;
pub mod json;
pub mod screen;

pub use diagnostic::{
    CompilationOutcome, CompilationStatus, Diagnostic, PromptKind, Severity, compilation_schema,
};
pub use image::{ImageProfile, ImagePromptRequest, compile_image_prompt, image_schema};

pub use interview::{
    InterviewOutcome, InterviewQuestion, InterviewRequest, InterviewStatus, interview_schema,
    run_interview,
};
