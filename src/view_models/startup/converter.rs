//! Converter setup wording from recorded backend facts (ADR 0066).

#![warn(clippy::pedantic)]

use crate::audio_format::probe::{
    ConverterObservation, ConverterProbe, ExecutableSource, ProbeOutcome,
};

pub(crate) const TITLE: &str = "Converter setup";
pub(crate) const HELP: &str = "Enter the FLAC executable path below, or leave it blank to find flac on PATH. Test converters checks this draft and PATH ffmpeg without saving or converting a track. Save correction preserves the original configuration. Reload file to edit again after saving.";
pub(crate) const INSTALLATION: &str = "To install FLAC or ffmpeg, use your Linux distribution's package tools and documentation. This app does not install software. After installation, press Test converters again. A newly available executable must be in the app's existing PATH, or entered as the FLAC path here.";

pub(crate) fn report(observation: &ConverterObservation) -> String {
    let policy = if observation.flac.available() {
        if observation.ffmpeg.available() {
            "FLAC is available for the first conversion attempt; ffmpeg is available as the existing fallback if FLAC rejects the input."
        } else {
            "FLAC is available for the first conversion attempt. The ffmpeg fallback did not pass its version check."
        }
    } else if observation.ffmpeg.available() {
        "FLAC did not pass its version check. The existing WAV download conversion path can use ffmpeg fallback. An explicitly configured FLAC executable is never replaced with PATH flac."
    } else {
        "Neither converter passed its version check. WAV downloads may retain the WAV with a conversion warning; other audio formats remain usable."
    };
    format!("{}\n{}\n{policy}\nApp tested executable versions only. No file conversion, download, configuration save or software installation was performed.\n\n", probe_report("FLAC", &observation.flac), probe_report("ffmpeg", &observation.ffmpeg))
}

fn probe_report(tool: &str, probe: &ConverterProbe) -> String {
    let source = match probe.source {
        ExecutableSource::Configured => "configured path",
        ExecutableSource::Path => "PATH",
    };
    let outcome = match probe.outcome {
        ProbeOutcome::Exited(Some(0)) => "version check succeeded (exit 0)".into(),
        ProbeOutcome::Exited(Some(code)) => format!("version check failed (exit {code})"),
        ProbeOutcome::Exited(None) => "version check terminated by a signal".into(),
        ProbeOutcome::Missing => "executable was not found".into(),
        ProbeOutcome::PermissionDenied => "permission to execute was denied".into(),
        ProbeOutcome::TimedOut => {
            "version check exceeded five seconds; app terminated and reaped the process".into()
        }
        ProbeOutcome::OutputLimit => {
            "version check reached the 16 KiB output limit; app terminated and reaped the process"
                .into()
        }
        ProbeOutcome::IoFailure(kind) => {
            format!("app could not complete the version check ({kind:?})")
        }
    };
    crate::diagnostics::redact_endpoint_details(&format!(
        "{} — App tested {tool} executable {} ({source}): {outcome}. Process output is omitted to protect credentials.",
        probe.recorded_at.format("%Y-%m-%d %H:%M:%S UTC"), probe.executable.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_converter_report_preserves_recorded_time_and_fallback_facts() {
        let time = chrono::DateTime::parse_from_rfc3339("2026-09-10T12:34:56Z")
            .unwrap()
            .to_utc();
        let mut observation = ConverterObservation {
            flac: ConverterProbe {
                executable: "/missing/flac".into(),
                source: ExecutableSource::Configured,
                recorded_at: time,
                outcome: ProbeOutcome::Missing,
            },
            ffmpeg: ConverterProbe {
                executable: "ffmpeg".into(),
                source: ExecutableSource::Path,
                recorded_at: time,
                outcome: ProbeOutcome::Exited(Some(0)),
            },
        };
        let text = report(&observation);
        assert!(text.contains("2026-09-10 12:34:56 UTC"));
        assert!(text.contains("/missing/flac (configured path)"));
        assert!(text.contains("ffmpeg (PATH)"));
        assert!(text.contains("can use ffmpeg fallback"));
        assert!(text.contains("never replaced with PATH flac"));
        observation.ffmpeg.outcome = ProbeOutcome::TimedOut;
        let text = report(&observation);
        assert!(text.contains("five seconds"));
        assert!(text.contains("may retain the WAV"));
        assert!(!text.contains("can use ffmpeg fallback"));
    }
}
