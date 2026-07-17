use crate::{importer, storage};
use serde_json::Value;
use std::fs;
use std::io::Write;

pub fn export_items(ids: &[String], format: &str, output_path: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let query = storage::HistoryQuery {
        id: None,
        keyword: None,
        min_score: None,
        max_score: None,
        favorite: None,
        page_size: Some(9999),
        page: Some(1),
    };
    let all_data = storage::get_history(query)?;
    let all_items: Vec<storage::HistoryItem> = serde_json::from_value(all_data["items"].clone()).unwrap_or_default();

    let id_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    let items: Vec<storage::HistoryItem> = all_items
        .iter()
        .filter(|i| id_set.contains(i.id.as_str()))
        .map(importer::sanitize_for_export)
        .collect();

    if items.is_empty() {
        return Err("No items found for export".into());
    }

    match format {
        "json" => export_json(&items, output_path)?,
        "csv" => export_csv(&items, output_path)?,
        "markdown" | "md" => export_markdown(&items, output_path)?,
        "txt" => export_txt(&items, output_path)?,
        "zip" => export_zip(&items, output_path)?,
        _ => return Err(format!("Unsupported format: {}", format).into()),
    }

    Ok(serde_json::json!({
        "exported": items.len(),
        "format": format,
        "path": output_path
    }))
}

fn export_json(items: &[storage::HistoryItem], path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(items)?;
    fs::write(path, json)?;
    Ok(())
}

fn export_csv(items: &[storage::HistoryItem], path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["ID", "File Name", "Source", "Quality Score", "Quality Label", "Prompt EN", "Prompt ZH", "Aspect Ratio", "Model", "Provider", "Elapsed (ms)", "Created At"])?;

    for item in items {
        wtr.write_record([
            &item.id,
            &item.file_name,
            &item.source_type,
            &item.quality_score.to_string(),
            &item.quality_label,
            item.prompt_en.as_deref().unwrap_or(""),
            item.prompt_zh.as_deref().unwrap_or(""),
            item.aspect_ratio.as_deref().unwrap_or(""),
            &item.model,
            &item.provider,
            &item.elapsed_ms.to_string(),
            &item.created_at.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn export_markdown(items: &[storage::HistoryItem], path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut out = String::new();
    out.push_str("# AutoPrompt Export\n\n");

    for (i, item) in items.iter().enumerate() {
        out.push_str(&format!("## {} — {}\n\n", i + 1, if item.file_name.is_empty() { &item.id } else { &item.file_name }));
        out.push_str(&format!("- **Quality:** {} ({})\n", item.quality_score, item.quality_label));

        if let Some(ar) = &item.aspect_ratio {
            out.push_str(&format!("- **Aspect Ratio:** {}\n", ar));
        }
        if let Some(cp) = item.contains_people {
            out.push_str(&format!("- **Contains People:** {}\n", cp));
        }
        out.push_str(&format!("- **Model:** {} ({})\n", item.model, item.provider));
        out.push_str(&format!("- **Time:** {}ms\n\n", item.elapsed_ms));

        if let Some(ref en) = item.prompt_en {
            out.push_str("### Prompt (EN)\n\n");
            out.push_str(en);
            out.push_str("\n\n");
        }
        if let Some(ref zh) = item.prompt_zh {
            out.push_str("### Prompt (ZH)\n\n");
            out.push_str(zh);
            out.push_str("\n\n");
        }
        if let Some(ref rp) = item.reconstructed_prompt {
            out.push_str("### Structured Prompt\n\n");
            out.push_str(&format!("```json\n{}\n```\n\n", serde_json::to_string_pretty(rp).unwrap_or_default()));
        }
        out.push_str("---\n\n");
    }

    fs::write(path, out)?;
    Ok(())
}

fn export_txt(items: &[storage::HistoryItem], path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut out = String::new();

    for (i, item) in items.iter().enumerate() {
        let name = if item.file_name.is_empty() { &item.id } else { &item.file_name };
        out.push_str(&format!("[{}] {}\n", i + 1, name));
        out.push_str(&format!("Quality: {} ({})\n", item.quality_score, item.quality_label));

        if let Some(ref en) = item.prompt_en {
            out.push_str(&format!("Prompt EN: {}\n", en));
        }
        if let Some(ref zh) = item.prompt_zh {
            out.push_str(&format!("Prompt ZH: {}\n", zh));
        }
        out.push_str("\n");
    }

    fs::write(path, out)?;
    Ok(())
}

fn export_zip(items: &[storage::HistoryItem], path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let json = serde_json::to_string_pretty(items)?;
    zip.start_file("data.json", options)?;
    zip.write_all(json.as_bytes())?;

    let thumbs = storage::thumbs_dir();
    for item in items {
        let thumb_path = thumbs.join(format!("{}.jpg", item.id));
        if thumb_path.exists() {
            if let Ok(data) = fs::read(&thumb_path) {
                zip.start_file(format!("thumbnails/{}.jpg", item.id), options)?;
                zip.write_all(&data)?;
            }
        }
    }

    zip.finish()?;
    Ok(())
}
