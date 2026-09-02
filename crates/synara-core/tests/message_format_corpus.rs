use std::{collections::HashSet, fs, path::PathBuf};

use matrix_sdk::ruma::events::room::message::{MessageType, TextMessageEventContent};
use serde::Deserialize;
use synara_core::app::{
    send::{message_content, MAX_OUTBOUND_TEXT_PAYLOAD_BYTES},
    timeline::project_formatted_body,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Corpus {
    schema_version: u32,
    outbound_text_payload_max_bytes: usize,
    presentation_formatted_body_max_bytes: usize,
    cases: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CorpusCase {
    id: String,
    body: String,
    formatted_body: String,
    generator: Option<Generator>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Generator {
    NestedTag {
        tag: String,
        count: usize,
        text: String,
    },
    RepeatedText {
        prefix: String,
        unit: String,
        count: usize,
        suffix: String,
    },
}

fn load_corpus() -> Corpus {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/future-projects/rust-ownership-expansion/fixtures/message-format/corpus.json",
    );
    serde_json::from_str(&fs::read_to_string(path).expect("shared corpus should be readable"))
        .expect("shared corpus should decode")
}

fn formatted_body(case: &CorpusCase) -> String {
    match &case.generator {
        None => case.formatted_body.clone(),
        Some(Generator::NestedTag { tag, count, text }) => format!(
            "{}{}{}",
            format!("<{tag}>").repeat(*count),
            text,
            format!("</{tag}>").repeat(*count)
        ),
        Some(Generator::RepeatedText {
            prefix,
            unit,
            count,
            suffix,
        }) => format!("{prefix}{}{suffix}", unit.repeat(*count)),
    }
}

#[test]
fn shared_corpus_keeps_core_projection_protocol_faithful_and_caps_outbound_text_payload() {
    let corpus = load_corpus();
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(
        corpus.outbound_text_payload_max_bytes,
        MAX_OUTBOUND_TEXT_PAYLOAD_BYTES
    );
    assert!(corpus.presentation_formatted_body_max_bytes > MAX_OUTBOUND_TEXT_PAYLOAD_BYTES);

    let mut ids = HashSet::new();
    for case in &corpus.cases {
        assert!(
            ids.insert(case.id.as_str()),
            "duplicate corpus id: {}",
            case.id
        );
        let html = formatted_body(case);
        let message = MessageType::Text(TextMessageEventContent::html(
            case.body.clone(),
            html.clone(),
        ));
        assert_eq!(
            project_formatted_body(&message).as_deref(),
            Some(html.as_str()),
            "Core must preserve untrusted protocol HTML for presenter-owned sanitization: {}",
            case.id
        );

        let result = message_content(
            case.body.clone(),
            None,
            Some(html.clone()),
            None,
            false,
            None,
            None,
        );
        if case.body.len() + html.len() <= corpus.outbound_text_payload_max_bytes {
            assert!(
                result.is_ok(),
                "bounded outbound case should pass: {}",
                case.id
            );
        } else {
            assert_eq!(
                result.expect_err("oversized outbound text payload must fail"),
                "d0.4-send-text-payload-too-large",
                "{}",
                case.id
            );
        }
    }
}
