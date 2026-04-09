use protocol::OcrEngine;

use super::{error::Error, raw};
use crate::config::UPSTREAM_ATTACHMENT_MAX_BYTES;
use crate::utils::blob::BlobReader;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::engine::Engine;

#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub data: BlobReader,
}

fn file_to_parts(
    file: File,
    capability: &super::Capability,
) -> Result<impl Iterator<Item = raw::MessagePart> + '_, Error> {
    if file.data.len() > UPSTREAM_ATTACHMENT_MAX_BYTES {
        return Err(Error::Incompatible(
            "Attachment exceeds upstream size limit (20 MiB)",
        ));
    }

    Ok(raw::MessagePart::from_file(file)
        .into_iter()
        .filter(move |part| match part.r#type {
            raw::MultiPartMessageType::ImageUrl => capability.image_input,
            raw::MultiPartMessageType::InputAudio => capability.audio,
            raw::MultiPartMessageType::File => capability.ocr != OcrEngine::Disabled,
            raw::MultiPartMessageType::Text => true,
        }))
}

/// Generated Image that haven't been stored
pub struct GeneratedImage {
    pub data: Vec<u8>,
    pub mime_type: String,
}

impl GeneratedImage {
    pub fn from_raw_image(raw: raw::Image) -> Result<Self, Error> {
        let raw::ImageUrl { url } = raw.image_url;
        let data_url = url
            .strip_prefix("data:")
            .ok_or_else(|| Error::Incompatible("Image URL missing data: prefix"))?;
        let (mime_part, base64_data) = data_url
            .split_once(';')
            .ok_or_else(|| Error::Incompatible("Image URL missing mime type"))?;
        let base64_data = base64_data
            .strip_prefix("base64,")
            .ok_or_else(|| Error::Incompatible("Image URL missing base64, prefix"))?;
        let data = BASE64_STANDARD
            .decode(base64_data)
            .map_err(|_| Error::Incompatible("Failed to decode base64 image"))?;
        Ok(Self {
            data,
            mime_type: mime_part.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MessageToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct MessageToolResult {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        content: String,
        annotations: Option<serde_json::Value>,
        reasoning_details: Option<serde_json::Value>,
        files: Vec<File>,
    },
    MultipartUser {
        text: String,
        files: Vec<File>,
    },
    ToolCall(MessageToolCall),
    ToolResult(MessageToolResult),
}

impl Message {
    pub fn to_raw_message(
        self,
        target_model_id: &str,
        capability: &super::Capability,
    ) -> Result<raw::Message, Error> {
        match self {
            Message::Assistant {
                content,
                annotations,
                reasoning_details,
                files,
            } => {
                let mut reasoning_details_value = None;
                if let Some(details) = reasoning_details {
                    if let Some(obj) = details.as_object() {
                        if let Some(model_id) = obj.get("model_id").and_then(|v| v.as_str()) {
                            if target_model_id.starts_with(model_id) {
                                reasoning_details_value = obj.get("data").cloned();
                            }
                        }
                    }
                }
                if files.is_empty() {
                    return Ok(raw::Message {
                        role: raw::Role::Assistant,
                        content: Some(content),
                        annotations,
                        reasoning_details: reasoning_details_value
                            .map(|v| vec![v])
                            .unwrap_or_default(),
                        ..Default::default()
                    });
                }
                let mut parts = Vec::new();

                for file in files {
                    parts.extend(file_to_parts(file, capability)?);
                }

                parts.push(raw::MessagePart::text(content));

                Ok(raw::Message {
                    role: raw::Role::Assistant,
                    contents: Some(parts),
                    annotations,
                    reasoning_details: reasoning_details_value.map(|v| vec![v]).unwrap_or_default(),
                    ..Default::default()
                })
            }
            Message::System(msg) => Ok(raw::Message {
                role: raw::Role::System,
                content: Some(msg),
                ..Default::default()
            }),
            Message::User(msg) => Ok(raw::Message {
                role: raw::Role::User,
                content: Some(msg),
                ..Default::default()
            }),

            Message::MultipartUser { text, files } => {
                let mut parts = vec![raw::MessagePart::text(text)];

                for file in files {
                    parts.extend(file_to_parts(file, capability)?);
                }

                Ok(raw::Message {
                    role: raw::Role::User,
                    contents: Some(parts),
                    ..Default::default()
                })
            }
            Message::ToolCall(MessageToolCall {
                id,
                name,
                arguments,
            }) => Ok(raw::Message {
                role: raw::Role::Assistant,
                tool_calls: Some(vec![raw::ToolCallReq {
                    id,
                    function: raw::ToolFunctionResp {
                        name: Some(name),
                        arguments: Some(arguments),
                    },
                    r#type: "function".to_string(),
                }]),
                content: Some("".to_string()),
                ..Default::default()
            }),
            Message::ToolResult(MessageToolResult { id, content }) => Ok(raw::Message {
                role: raw::Role::Tool,
                content: Some(content),
                tool_call_id: Some(id),
                ..Default::default()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::blob::BlobDB;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_blob_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("ichat-web-{name}-{}-{nanos}.redb", std::process::id()))
    }

    async fn make_blob_reader(size: usize) -> BlobReader {
        let path = temp_blob_path("attachment-limit");
        let blob = BlobDB::new(Arc::new(redb::Database::create(&path).expect("db should create")));
        let bytes = bytes::Bytes::from(vec![b'a'; size]);
        blob.insert(1, size, tokio_stream::iter(vec![bytes]))
            .await
            .expect("blob should insert");
        let reader = blob.get(1).expect("blob should exist");
        std::fs::remove_file(path).expect("temp db should be removed");
        reader.into()
    }

    #[tokio::test]
    async fn rejects_attachment_that_exceeds_upstream_limit() {
        let capability = super::super::Capability {
            text_output: true,
            image_output: false,
            image_input: true,
            structured_output: false,
            toolcall: false,
            ocr: OcrEngine::Native,
            audio: true,
            reasoning: false,
            reasoning_effort: Default::default(),
        };

        let message = Message::MultipartUser {
            text: "check this file".to_string(),
            files: vec![File {
                name: "big.pdf".to_string(),
                data: make_blob_reader(UPSTREAM_ATTACHMENT_MAX_BYTES + 1).await,
            }],
        };

        let error = match message.to_raw_message("test-model", &capability) {
            Ok(_) => panic!("oversized attachment should be rejected"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::Incompatible("Attachment exceeds upstream size limit (20 MiB)")
        ));
    }
}

impl From<protocol::ModelCapability> for super::MaybeCapability {
    fn from(capability: protocol::ModelCapability) -> Self {
        super::MaybeCapability {
            text_output: None,
            image_output: capability.image,
            image_input: None,
            structured_output: capability.json,
            toolcall: capability.tool,
            ocr: capability.ocr,
            audio: capability.audio,
            reasoning: capability.reasoning.map(|r| r.is_enabled()),
            reasoning_effort: capability.reasoning.map(|r| r.effort()),
        }
    }
}
