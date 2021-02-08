use serde::{Deserialize, Serialize};

use lsp_types::MarkedString;
use lsp_types::Position;
use lsp_types::Url;

#[derive(Serialize, Deserialize)]
pub struct Element {
    pub id: u64,
    pub el_type: ElementType,
}

#[derive(Serialize, Deserialize)]
pub enum ElementType {
    Vertex,
    Edge,
}

#[derive(Serialize, Deserialize)]
pub struct Vertex {
    #[serde(flatten)]
    pub el: Element,
    pub label: VertexLabel,
}

#[derive(Serialize, Deserialize)]
pub enum VertexLabel {
    Metadata,
    Project,
    Range,
    Location,
    Document,
    Moniker,
    PackageInfo,
    ResultSet,
    DocumentSymbolResult,
    FoldingRangeResult,
    DiagnosticResult,
    DeclarationResult,
    DefinitionResult,
    TypeDefinitionResult,
    HoverResult,
    ReferenceResult,
    ImplementationResult,
}

#[derive(Serialize, Deserialize)]
pub struct Edge {
    #[serde(flatten)]
    pub el: Element,
    pub label: EdgeLabel,
}

#[derive(Serialize, Deserialize)]
pub enum EdgeLabel {
    Contains,
    Item,
    Next,
    Moniker,
    NextMoniker,
    PackageInfo,
    TextDocDocumentSymbol,
    TextDocFoldingRange,
    TextDocDocumentLink,
    TextDocDiagnostic,
    TextDocDefinition,
    TextDocDeclaration,
    TextDocTypeDefinition,
    TextDocHover,
    TextDocReferences,
    TextDocImplementation,
}

#[derive(Serialize, Deserialize)]
pub struct Contains {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_vs: Vec<u64>,
}

impl Contains {
    pub fn new(id: u64, out_v: u64, in_vs: Vec<u64>) -> Contains {
        Contains {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::Contains,
            },
            out_v,
            in_vs,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DefinitionResult {
    #[serde(flatten)]
    pub vertex: Vertex,
}

impl DefinitionResult {
    pub fn new(id: u64) -> DefinitionResult {
        DefinitionResult {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::DefinitionResult,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TextDocumentDefinition {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl TextDocumentDefinition {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> TextDocumentDefinition {
        TextDocumentDefinition {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::TextDocDefinition,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Document {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub uri: Url,
    pub language_id: String,
}

impl Document {
    pub fn new(id: u64, language_id: &str, uri: &str) -> Document {
        Document {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::Document,
            },
            uri: Url::parse(uri).expect("passed uri was not valid"),
            language_id: String::from(language_id),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HoverResult {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub result: HoverResultContent,
}

#[derive(Serialize, Deserialize)]
pub struct HoverResultContent {
    pub contents: Vec<MarkedString>,
}

impl HoverResult {
    pub fn new(id: u64, contents: Vec<MarkedString>) -> HoverResult {
        HoverResult {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::HoverResult,
            },
            result: HoverResultContent { contents },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TextDocumentHover {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl TextDocumentHover {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> TextDocumentHover {
        TextDocumentHover {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::TextDocHover,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_vs: Vec<u64>,
    pub document: u64,
    pub property: String,
}

impl Item {
    pub fn new(id: u64, out_v: u64, in_vs: Vec<u64>, document: u64) -> Item {
        Item::new_with_property(id, out_v, in_vs, document, "")
    }

    pub fn new_with_property<T: Into<String>>(
        id: u64,
        out_v: u64,
        in_vs: Vec<u64>,
        document: u64,
        property: T,
    ) -> Item {
        Item {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::Item,
            },
            out_v,
            in_vs,
            document,
            property: property.into(),
        }
    }

    pub fn new_of_definition(id: u64, out_v: u64, in_vs: Vec<u64>, document: u64) -> Item {
        Item::new_with_property(id, out_v, in_vs, document, "definitions")
    }

    pub fn new_of_references(id: u64, out_v: u64, in_vs: Vec<u64>, document: u64) -> Item {
        Item::new_with_property(id, out_v, in_vs, document, "references")
    }
}

const VERSION: &'static str = "0.4.3";
const POSITION_ENCODING: &'static str = "utf-16";

#[derive(Serialize, Deserialize)]
pub struct MetaData {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub version: &'static str,
    pub project_root: String,
    pub position_encoding: &'static str,
    pub tool_info: ToolInfo,
}

#[derive(Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: &'static str,
    pub args: Vec<String>,
}

impl MetaData {
    pub fn new(id: u64, root: String, info: ToolInfo) -> MetaData {
        MetaData {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::Metadata,
            },
            version: VERSION,
            project_root: root,
            position_encoding: POSITION_ENCODING,
            tool_info: info,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Moniker {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub kind: String,
    pub scheme: String,
    pub identifier: String,
}

impl Moniker {
    pub fn new<T: Into<String>>(id: u64, kind: T, scheme: T, identifier: T) -> Moniker {
        Moniker {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::Moniker,
            },
            kind: kind.into(),
            scheme: scheme.into(),
            identifier: identifier.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MonikerEdge {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl MonikerEdge {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> MonikerEdge {
        MonikerEdge {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::Moniker,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NextMonikerEdge {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl NextMonikerEdge {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> NextMonikerEdge {
        NextMonikerEdge {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::NextMoniker,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Next {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl Next {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> Next {
        Next {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::Next,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PackageInfo {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub name: String,
    pub manager: String,
    pub version: String,
}

impl PackageInfo {
    pub fn new<T: Into<String>>(id: u64, name: T, manager: T, version: T) -> PackageInfo {
        PackageInfo {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::PackageInfo,
            },
            name: name.into(),
            manager: manager.into(),
            version: version.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PackageInfoEdge {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl PackageInfoEdge {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> PackageInfoEdge {
        PackageInfoEdge {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::PackageInfo,
            },
            out_v,
            in_v,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub kind: String,
}

impl Project {
    pub fn new<T: Into<String>>(id: u64, language_id: T) -> Project {
        Project {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::Project,
            },
            kind: language_id.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Range {
    #[serde(flatten)]
    pub vertex: Vertex,
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(id: u64, start: Position, end: Position) -> Range {
        Range {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::Range,
            },
            start,
            end,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ResultSet {
    #[serde(flatten)]
    pub vertex: Vertex,
}

impl ResultSet {
    pub fn new(id: u64) -> ResultSet {
        ResultSet {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::ResultSet,
            },
        }
    }

    pub fn new_reference_result(id: u64) -> ResultSet {
        ResultSet {
            vertex: Vertex {
                el: Element {
                    id,
                    el_type: ElementType::Vertex,
                },
                label: VertexLabel::ReferenceResult,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TextDocumentReferences {
    #[serde(flatten)]
    pub edge: Edge,
    pub out_v: u64,
    pub in_v: u64,
}

impl TextDocumentReferences {
    pub fn new(id: u64, out_v: u64, in_v: u64) -> TextDocumentReferences {
        TextDocumentReferences {
            edge: Edge {
                el: Element {
                    id,
                    el_type: ElementType::Edge,
                },
                label: EdgeLabel::TextDocReferences,
            },
            out_v,
            in_v,
        }
    }
}
