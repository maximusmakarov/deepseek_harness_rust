#![forbid(unsafe_code)]
//! Office document tools: read and create docx/xlsx/pptx.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use async_trait::async_trait;
use calamine::{Data, Reader, open_workbook_auto};
use dshr_tools::{Tool, ToolCtx, ToolError, ToolOutcome};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OfficeError {
    #[error("office I/O: {0}")]
    Io(String),

    #[error("unsupported office format: {0}")]
    Unsupported(String),

    #[error("office parse: {0}")]
    Parse(String),

    #[error("office write: {0}")]
    Write(String),
}

#[must_use]
pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn read_text(path: &Path) -> Result<String, OfficeError> {
    match extension(path).as_str() {
        "docx" => read_docx(path),
        "xlsx" | "xls" | "ods" => read_xlsx(path),
        "pptx" => read_pptx(path),
        other => Err(OfficeError::Unsupported(other.to_owned())),
    }
}

pub fn read_docx(path: &Path) -> Result<String, OfficeError> {
    let file = File::open(path).map_err(|error| OfficeError::Io(error.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| OfficeError::Parse(error.to_string()))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|error| OfficeError::Parse(format!("word/document.xml: {error}")))?;
    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|error| OfficeError::Io(error.to_string()))?;

    let mut reader = XmlReader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut in_text = false;
    let mut line = String::new();
    let mut output = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == "w:t" => in_text = true,
            Ok(Event::End(event)) if event.name().as_ref() == "w:t" => in_text = false,
            Ok(Event::Text(text)) if in_text => {
                line.push_str(text.as_ref());
            }
            Ok(Event::End(event)) if event.name().as_ref() == "w:p" => {
                output.push_str(line.trim());
                output.push('\n');
                line.clear();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(OfficeError::Parse(error.to_string())),
            _ => {}
        }
    }
    Ok(output)
}

pub fn read_xlsx(path: &Path) -> Result<String, OfficeError> {
    let mut workbook =
        open_workbook_auto(path).map_err(|error| OfficeError::Parse(error.to_string()))?;
    let mut output = String::new();
    for name in workbook.sheet_names().to_owned() {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|error| OfficeError::Parse(error.to_string()))?;
        output.push_str(&format!("## {name}\n"));
        for row in range.rows() {
            let cells = row.iter().map(cell_text).collect::<Vec<_>>().join("\t");
            if !cells.trim().is_empty() {
                output.push_str(&cells);
                output.push('\n');
            }
        }
    }
    Ok(output)
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.clone(),
        Data::Float(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(value) => format!("#{value:?}"),
    }
}

pub fn read_pptx(path: &Path) -> Result<String, OfficeError> {
    let file = File::open(path).map_err(|error| OfficeError::Io(error.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| OfficeError::Parse(error.to_string()))?;
    let mut slides = archive
        .file_names()
        .filter_map(|name| name.ok())
        .filter_map(|name| slide_number(&name).map(|number| (number, name.into_owned())))
        .collect::<Vec<_>>();
    slides.sort_by_key(|(number, _)| *number);

    let mut output = String::new();
    for (number, name) in slides {
        let mut entry = archive
            .by_name(&name)
            .map_err(|error| OfficeError::Parse(error.to_string()))?;
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|error| OfficeError::Io(error.to_string()))?;
        output.push_str(&format!("## Slide {number}\n"));
        output.push_str(&pptx_text(&xml)?);
    }
    Ok(output)
}

fn slide_number(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("ppt/slides/slide")?;
    let number = rest.strip_suffix(".xml")?;
    number.parse().ok()
}

fn pptx_text(xml: &str) -> Result<String, OfficeError> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut in_text = false;
    let mut line = String::new();
    let mut output = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == "a:t" => in_text = true,
            Ok(Event::End(event)) if event.name().as_ref() == "a:t" => in_text = false,
            Ok(Event::Text(text)) if in_text => line.push_str(text.as_ref()),
            Ok(Event::End(event)) if event.name().as_ref() == "a:p" => {
                if !line.trim().is_empty() {
                    output.push_str(line.trim());
                    output.push('\n');
                }
                line.clear();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(OfficeError::Parse(error.to_string())),
            _ => {}
        }
    }
    Ok(output)
}

pub fn create_docx(path: &Path, title: &str, paragraphs: &[String]) -> Result<(), OfficeError> {
    let mut body = String::new();
    if !title.trim().is_empty() {
        body.push_str(&docx_paragraph(title, true));
    }
    for paragraph in paragraphs {
        body.push_str(&docx_paragraph(paragraph, false));
    }
    body.push_str(r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr>"#);
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    );
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

    let file = create_file(path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in [
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", document.as_bytes()),
    ] {
        writer
            .start_file(name, options)
            .map_err(|error| OfficeError::Write(error.to_string()))?;
        std::io::Write::write_all(&mut writer, data)
            .map_err(|error| OfficeError::Write(error.to_string()))?;
    }
    writer
        .finish()
        .map_err(|error| OfficeError::Write(error.to_string()))?;
    Ok(())
}

fn docx_paragraph(text: &str, bold: bool) -> String {
    let run_properties = if bold { r#"<w:rPr><w:b/></w:rPr>"# } else { "" };
    format!(
        r#"<w:p><w:r>{run_properties}<w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        quick_xml::escape::escape(text)
    )
}

pub fn create_xlsx(path: &Path, sheets: &[(String, Vec<Vec<String>>)]) -> Result<(), OfficeError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    for (name, rows) in sheets {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(name)
            .map_err(|error| OfficeError::Write(error.to_string()))?;
        for (row_index, row) in rows.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                worksheet
                    .write_string(row_index as u32, column_index as u16, value)
                    .map_err(|error| OfficeError::Write(error.to_string()))?;
            }
        }
    }
    workbook
        .save(path)
        .map_err(|error| OfficeError::Write(error.to_string()))
}

pub fn create_pptx(
    path: &Path,
    title: &str,
    slides: &[(String, Vec<String>)],
) -> Result<(), OfficeError> {
    let mut template = zip::ZipArchive::new(std::io::Cursor::new(DEFAULT_PPTX))
        .map_err(|error| OfficeError::Parse(error.to_string()))?;
    let mut entries = std::collections::BTreeMap::new();
    let names = template
        .file_names()
        .filter_map(|name| name.ok().map(std::borrow::Cow::into_owned))
        .collect::<Vec<_>>();
    for name in names {
        let mut entry = template
            .by_name(&name)
            .map_err(|error| OfficeError::Parse(error.to_string()))?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| OfficeError::Io(error.to_string()))?;
        entries.insert(name, bytes);
    }

    let mut texts = Vec::new();
    if !title.trim().is_empty() {
        texts.push(title.to_owned());
    }
    for (heading, bullets) in slides {
        let mut text = heading.clone();
        for bullet in bullets {
            text.push('\n');
            text.push_str(bullet);
        }
        texts.push(text);
    }
    if texts.is_empty() {
        texts.push(String::new());
    }

    let count = texts.len();
    for (index, text) in texts.iter().enumerate() {
        let number = index + 1;
        entries.insert(
            format!("ppt/slides/slide{number}.xml"),
            slide_xml(text).into_bytes(),
        );
        entries.insert(
            format!("ppt/slides/_rels/slide{number}.xml.rels"),
            slide_rels().as_bytes().to_vec(),
        );
    }

    let content_types = String::from_utf8(
        entries
            .remove("[Content_Types].xml")
            .ok_or_else(|| OfficeError::Parse("template has no [Content_Types].xml".into()))?,
    )
    .map_err(|error| OfficeError::Parse(error.to_string()))?;
    entries.insert(
        "[Content_Types].xml".into(),
        add_slide_content_types(&content_types, count).into_bytes(),
    );

    let presentation = String::from_utf8(
        entries
            .remove("ppt/presentation.xml")
            .ok_or_else(|| OfficeError::Parse("template has no presentation.xml".into()))?,
    )
    .map_err(|error| OfficeError::Parse(error.to_string()))?;
    entries.insert(
        "ppt/presentation.xml".into(),
        add_slide_ids(&presentation, count).into_bytes(),
    );

    let rels = String::from_utf8(
        entries
            .remove("ppt/_rels/presentation.xml.rels")
            .ok_or_else(|| OfficeError::Parse("template has no presentation rels".into()))?,
    )
    .map_err(|error| OfficeError::Parse(error.to_string()))?;
    entries.insert(
        "ppt/_rels/presentation.xml.rels".into(),
        add_slide_rels(&rels, count).into_bytes(),
    );

    let file = create_file(path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer
            .start_file(name, options)
            .map_err(|error| OfficeError::Write(error.to_string()))?;
        std::io::Write::write_all(&mut writer, &bytes)
            .map_err(|error| OfficeError::Write(error.to_string()))?;
    }
    writer
        .finish()
        .map_err(|error| OfficeError::Write(error.to_string()))?;
    Ok(())
}

const DEFAULT_PPTX: &[u8] = include_bytes!("../assets/default.pptx");

fn slide_xml(text: &str) -> String {
    let mut paragraphs = String::new();
    for line in text.lines() {
        paragraphs.push_str(&format!(
            r#"<a:p><a:r><a:rPr lang="en-US"/><a:t>{}</a:t></a:r></a:p>"#,
            quick_xml::escape::escape(line)
        ));
    }
    if paragraphs.is_empty() {
        paragraphs.push_str(r#"<a:p><a:endParaRPr lang="en-US"/></a:p>"#);
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Content"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="609600" y="609600"/><a:ext cx="7315200" cy="5029200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square"/><a:lstStyle/>{paragraphs}</p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    )
}

fn slide_rels() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/></Relationships>"#
}

fn add_slide_content_types(xml: &str, count: usize) -> String {
    let mut overrides = String::new();
    for number in 1..=count {
        overrides.push_str(&format!(
            r#"<Override PartName="/ppt/slides/slide{number}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#
        ));
    }
    xml.replacen("</Types>", &format!("{overrides}</Types>"), 1)
}

fn add_slide_ids(xml: &str, count: usize) -> String {
    let mut ids = String::from("<p:sldIdLst>");
    for number in 1..=count {
        ids.push_str(&format!(
            r#"<p:sldId id="{}" r:id="rId{}"/>"#,
            255 + number,
            100 + number
        ));
    }
    ids.push_str("</p:sldIdLst>");
    xml.replacen(
        "</p:sldMasterIdLst>",
        &format!("</p:sldMasterIdLst>{ids}"),
        1,
    )
}

fn add_slide_rels(xml: &str, count: usize) -> String {
    let mut rels = String::new();
    for number in 1..=count {
        rels.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{number}.xml"/>"#,
            100 + number
        ));
    }
    xml.replacen("</Relationships>", &format!("{rels}</Relationships>"), 1)
}

fn create_file(path: &Path) -> Result<File, OfficeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| OfficeError::Io(error.to_string()))?;
    }
    File::create(path).map_err(|error| OfficeError::Io(error.to_string()))
}

fn path_arg(ctx: &ToolCtx, args: &Value) -> Result<std::path::PathBuf, ToolError> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidArgs("path is required".into()))?;
    Ok(dshr_tools::resolve_path(&ctx.cwd, path))
}

pub struct OfficeReadTool;

#[async_trait]
impl Tool for OfficeReadTool {
    fn name(&self) -> &str {
        "office_read"
    }

    fn description(&self) -> &str {
        "Read text from a docx, xlsx, or pptx file."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {"path": {"type": "string", "minLength": 1}}
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let text = read_text(&path).map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(if text.trim().is_empty() {
            "(empty document)".to_owned()
        } else {
            text
        }))
    }
}

pub struct OfficeCreateDocxTool;

#[async_trait]
impl Tool for OfficeCreateDocxTool {
    fn name(&self) -> &str {
        "office_create_docx"
    }

    fn description(&self) -> &str {
        "Create a docx document with a title and paragraphs."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "title": {"type": "string"},
                "paragraphs": {"type": "array", "items": {"type": "string"}}
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let title = args["title"].as_str().unwrap_or("");
        let paragraphs = string_array(&args["paragraphs"]);
        create_docx(&path, title, &paragraphs)
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("created {}", path.display())))
    }
}

pub struct OfficeCreateXlsxTool;

#[async_trait]
impl Tool for OfficeCreateXlsxTool {
    fn name(&self) -> &str {
        "office_create_xlsx"
    }

    fn description(&self) -> &str {
        "Create an xlsx workbook from named sheets of string rows."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "sheets"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "sheets": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["name", "rows"],
                        "additionalProperties": false,
                        "properties": {
                            "name": {"type": "string", "minLength": 1},
                            "rows": {
                                "type": "array",
                                "items": {"type": "array", "items": {"type": "string"}}
                            }
                        }
                    }
                }
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let mut sheets = Vec::new();
        for sheet in args["sheets"].as_array().unwrap_or(&Vec::new()) {
            let name = sheet["name"].as_str().unwrap_or("Sheet").to_owned();
            let rows = sheet["rows"]
                .as_array()
                .map(|rows| rows.iter().map(string_array).collect::<Vec<_>>())
                .unwrap_or_default();
            sheets.push((name, rows));
        }
        if sheets.is_empty() {
            return Err(ToolError::InvalidArgs(
                "at least one sheet is required".into(),
            ));
        }
        create_xlsx(&path, &sheets).map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("created {}", path.display())))
    }
}

pub struct OfficeCreatePptxTool;

#[async_trait]
impl Tool for OfficeCreatePptxTool {
    fn name(&self) -> &str {
        "office_create_pptx"
    }

    fn description(&self) -> &str {
        "Create a pptx presentation with a title and slides of bullets."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "title": {"type": "string"},
                "slides": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["title"],
                        "additionalProperties": false,
                        "properties": {
                            "title": {"type": "string"},
                            "bullets": {"type": "array", "items": {"type": "string"}}
                        }
                    }
                }
            }
        })
    }

    async fn execute(&self, ctx: &ToolCtx, args: Value) -> Result<ToolOutcome, ToolError> {
        let path = path_arg(ctx, &args)?;
        let title = args["title"].as_str().unwrap_or("");
        let mut slides = Vec::new();
        for slide in args["slides"].as_array().unwrap_or(&Vec::new()) {
            slides.push((
                slide["title"].as_str().unwrap_or("").to_owned(),
                string_array(&slide["bullets"]),
            ));
        }
        create_pptx(&path, title, &slides).map_err(|error| ToolError::Failed(error.to_string()))?;
        Ok(ToolOutcome::text(format!("created {}", path.display())))
    }
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

pub fn register_tools(registry: &mut dshr_tools::ToolRegistry) {
    registry.register(crate::OfficeReadTool);
    registry.register(crate::OfficeCreateDocxTool);
    registry.register(crate::OfficeCreateXlsxTool);
    registry.register(crate::OfficeCreatePptxTool);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reads_docx() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.docx");
        create_docx(&path, "Title", &["Hello".into(), "World".into()]).expect("create");
        let text = read_docx(&path).expect("read");
        assert!(text.contains("Title"), "{text}");
        assert!(text.contains("Hello"), "{text}");
    }

    #[test]
    fn creates_and_reads_xlsx() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.xlsx");
        create_xlsx(
            &path,
            &[(
                "Data".into(),
                vec![
                    vec!["name".into(), "value".into()],
                    vec!["a".into(), "1".into()],
                ],
            )],
        )
        .expect("create");
        let text = read_xlsx(&path).expect("read");
        assert!(text.contains("Data"), "{text}");
        assert!(text.contains("value"), "{text}");
    }

    #[test]
    fn creates_and_reads_pptx() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.pptx");
        create_pptx(
            &path,
            "Deck",
            &[("First".into(), vec!["one".into(), "two".into()])],
        )
        .expect("create");
        let text = read_pptx(&path).expect("read");
        assert!(text.contains("Deck"), "{text}");
        assert!(text.contains("First"), "{text}");
    }
}
