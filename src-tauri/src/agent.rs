use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

const EMBED_DIM: usize = 384;
const MAX_FILE_BYTES: u64 = 500_000;
const INDEX_FILE: &str = "doc_index.bin";
const READABLE_EXTS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "json", "toml",
    "yaml", "yml", "html", "css", "xml", "csv", "log",
];

#[derive(Serialize, Deserialize)]
struct DocEntry {
    path: String,
    embedding: Vec<f32>,
}

fn index_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("personalproject").join(INDEX_FILE)
}

fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users"))
}

fn init_embedder() -> anyhow::Result<TextEmbedding> {
    Ok(TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2),
    )?)
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Walk the home directory and embed all readable files into a flat binary index.
/// Downloads the AllMiniLML6V2 model (~25 MB) on first run.
pub fn index_filesystem() -> anyhow::Result<()> {
    let mut model = init_embedder()?;

    let mut texts: Vec<String> = Vec::new();
    let mut paths: Vec<String> = Vec::new();

    for entry in WalkDir::new(home_dir())
        .max_depth(6)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !READABLE_EXTS.contains(&ext) {
            continue;
        }
        if path.metadata().map(|m| m.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        paths.push(path.to_string_lossy().into_owned());
        texts.push(content.chars().take(2000).collect());
    }

    if paths.is_empty() {
        return Ok(());
    }

    let embeddings: Vec<Vec<f32>> = model.embed(texts, None)?;

    let index: Vec<DocEntry> = paths
        .into_iter()
        .zip(embeddings)
        .map(|(path, embedding)| DocEntry { path, embedding })
        .collect();

    let out = index_path();
    std::fs::create_dir_all(out.parent().unwrap())?;
    let bytes = bincode::serialize(&index)?;
    std::fs::write(out, bytes)?;

    Ok(())
}

/// Embed `query`, scan the index for the closest document, and open it.
pub fn find_file(_app: tauri::AppHandle, query: String) -> anyhow::Result<String> {
    if query.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }

    let idx_path = index_path();
    if !idx_path.exists() {
        anyhow::bail!("File index not found — call index_filesystem() first");
    }

    let bytes = std::fs::read(&idx_path)?;
    let index: Vec<DocEntry> = bincode::deserialize(&bytes)?;

    if index.is_empty() {
        anyhow::bail!("File index is empty");
    }

    let mut model = init_embedder()?;
    let query_vec: Vec<f32> = model.embed(vec![query], None)?.remove(0);

    let best = index
        .iter()
        .map(|doc| (doc, cosine_sim(&query_vec, &doc.embedding)))
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(doc, _)| doc.path.clone())
        .ok_or_else(|| anyhow::anyhow!("No matching file found"))?;

    open::that(&best)?;
    Ok(best)
}

pub fn open_app(_app: tauri::AppHandle, url: String) -> anyhow::Result<()> {
    if url.is_empty() {
        anyhow::bail!("URL cannot be empty");
    }
    open::that(url)?;
    Ok(())
}

pub fn open_file(_app: tauri::AppHandle, filepath: String) -> anyhow::Result<()> {
    if filepath.is_empty() {
        anyhow::bail!("File path cannot be empty");
    }
    open::that(filepath)?;
    Ok(())
}

pub fn read_email(_app: tauri::AppHandle) -> anyhow::Result<()> {
    anyhow::bail!("Email reading not implemented")
}

pub fn search_web(_app: tauri::AppHandle, query: String) -> anyhow::Result<()> {
    if query.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }
    let url = format!(
        "https://www.google.com/search?q={}",
        urlencoding::encode(&query)
    );
    open::that(url)?;
    Ok(())
}
