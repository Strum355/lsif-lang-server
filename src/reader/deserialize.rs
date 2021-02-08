use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use lsp_types::{Range as LSRange, Url};

use super::interner::Interner;
use super::types::*;

use lazy_static::lazy_static;

use std::collections::HashMap;

type Deserializer = fn(&[u8]) -> Result<Payload>;

lazy_static! {
    static ref VERTEX_DESERIALIZERS: HashMap<&'static str, Deserializer> = [
        ("metaData", deserialize_metadata as Deserializer),
        ("document", deserialize_document as Deserializer),
        ("range", deserialize_range as Deserializer),
        ("hoverResult", deserialize_hover as Deserializer),
        ("moniker", deserialize_moniker as Deserializer),
        (
            "packageInformation",
            deserialize_package_info as Deserializer
        ),
        ("diagnosticResult", deserialize_diagnostics as Deserializer)
    ]
    .iter()
    .cloned()
    .collect();
}

pub fn deserialize_element(interner: &Interner, line: &[u8]) -> Result<Element> {
    #[derive(Deserialize, Serialize)]
    struct JSONPayload {
        //#[serde(borrow)]
        id: Value,
        #[serde(rename = "type")]
        el_type: String,
        label: String,
    }

    let payload: JSONPayload = serde_json::from_slice(line)?;

    let id = if payload.id.is_string() {
        interner.intern(payload.id.as_str().unwrap().as_bytes())?
    } else {
        // better be int
        payload.id.as_u64().unwrap()
    };

    let element = Element {
        id,
        el_type: payload.el_type.clone(),
        label: payload.label.clone(),
        payload: if payload.el_type == "edge" {
            Some(deserialize_edge(interner, line)?)
        } else if let Some(func) = VERTEX_DESERIALIZERS.get(payload.label.as_str()) {
            Some(func(line)?)
        } else {
            None
        },
    };

    Ok(element)
}

fn deserialize_edge(interner: &Interner, line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct EdgePayload {
        #[serde(rename = "outV")]
        out_v: Value,
        #[serde(rename = "inV")]
        in_v: Option<Value>,
        #[serde(rename = "inVs")]
        in_vs: Option<Vec<Value>>,
        #[serde(rename = "document")]
        document: Option<Value>,
    }

    let payload: EdgePayload = serde_json::from_slice(line)?;

    let out_v = if payload.out_v.is_string() {
        interner.intern(payload.out_v.as_str().unwrap().as_bytes())?
    } else {
        payload.out_v.as_u64().unwrap()
    };

    let in_v = if let Some(in_v) = payload.in_v {
        let in_v = if let Value::String(in_v) = in_v {
            interner.intern(in_v.as_bytes())?
        } else {
            in_v.as_u64().unwrap()
        };
        in_v
    } else {
        0 as u64
    };

    let document = if let Some(document) = payload.document {
        let document = if let Value::String(document) = document {
            interner.intern(document.as_bytes())?
        } else {
            document.as_u64().unwrap()
        };
        document
    } else {
        0 as u64
    };

    let in_vs = payload.in_vs.map_or_else(
        || Vec::new(),
        |in_vs| {
            in_vs
                .iter()
                .map(|v| {
                    let v = if let Value::String(v) = v {
                        interner.intern(v.as_bytes()).unwrap()
                    } else {
                        v.as_u64().unwrap()
                    };
                    v
                })
                .collect::<Vec<u64>>()
        },
    );

    Ok(Payload::Edge(Edge {
        out_v,
        in_v,
        in_vs,
        document,
    }))
}

fn deserialize_metadata(line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct MetaPayload {
        version: String,
        #[serde(rename = "projectRoot")]
        project_root: String,
    }

    let payload: MetaPayload = serde_json::from_slice(line)?;

    Ok(Payload::MetaData(MetaData {
        version: payload.version,
        project_root: payload.project_root,
    }))
}

fn deserialize_document(line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct DocumentPayload {
        uri: Url,
    }

    let payload: DocumentPayload = serde_json::from_slice(line)?;

    Ok(Payload::Document(payload.uri))
}

fn deserialize_range(line: &[u8]) -> Result<Payload> {
    let payload: LSRange = serde_json::from_slice(line)?;

    Ok(Payload::Range(Range {
        start_line: payload.start.line,
        start_character: payload.start.character,
        end_line: payload.end.line,
        end_character: payload.end.character,
    }))
}

fn deserialize_hover(line: &[u8]) -> Result<Payload> {
    Err(anyhow::anyhow!("asd").into())
}

fn deserialize_moniker(line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct MonikerPayload {
        kind: String,
        scheme: String,
        identifier: String,
    }

    let mut payload: MonikerPayload = serde_json::from_slice(line)?;

    if payload.scheme == "" {
        payload.scheme = "local".into()
    }

    Ok(Payload::Moniker(Moniker {
        kind: payload.kind,
        scheme: payload.scheme,
        identifier: payload.identifier,
    }))
}

fn deserialize_package_info(line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct PackageInfoPayload {
        name: String,
        version: String,
    }

    let payload: PackageInfoPayload = serde_json::from_slice(line)?;

    Ok(Payload::PackageInformation(PackageInformation {
        name: payload.name,
        version: payload.version,
    }))
}

fn deserialize_diagnostics(line: &[u8]) -> Result<Payload> {
    #[derive(Deserialize, Serialize)]
    struct DiagnosticPayload {
        name: String,
        version: String,
    }

    let payload: DiagnosticPayload = serde_json::from_slice(line)?;

    Ok(Payload::Diagnostics(Vec::new()))
}
