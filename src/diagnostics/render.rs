#![allow(dead_code)]

use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use ariadne::{Color, Label, Report, ReportKind, Source};

pub fn print_diagnostic(diagnostic: &Diagnostic, file_name: &str, source: &str) {
    let kind = match diagnostic.level {
        DiagnosticLevel::Error => ReportKind::Error,
        DiagnosticLevel::Warning => ReportKind::Warning,
    };
    let mut report = Report::build(
        kind,
        (file_name, diagnostic.span.start..diagnostic.span.end),
    )
    .with_message(diagnostic.message.clone());
    report = report.with_label(
        Label::new((file_name, diagnostic.span.start..diagnostic.span.end))
            .with_message(diagnostic.message.clone())
            .with_color(Color::Red),
    );
    for label in &diagnostic.labels {
        report = report.with_label(
            Label::new((file_name, diagnostic.span.start..diagnostic.span.end))
                .with_message(label.message.clone())
                .with_color(Color::Yellow),
        )
    }
    for note in &diagnostic.notes {
        report = report.with_note(note.clone());
    }
    if let Some(help) = &diagnostic.help {
        report = report.with_help(help.clone());
    }
    report
        .finish()
        .print((file_name, Source::from(source)))
        .unwrap();
}
