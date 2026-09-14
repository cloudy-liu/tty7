//! URL interpretation for a document with an explicit owning Host. Relative
//! resources never become a local `file:` URL, even for remote documents.

use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    Web(String),
    Anchor(String),
    File {
        path: PathBuf,
        fragment: Option<String>,
    },
}

fn decode(value: &str) -> Result<String, String> {
    let value = percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .map_err(|_| "link is not valid UTF-8")?
        .into_owned();
    if value.chars().any(char::is_control) {
        return Err("link contains a control character".into());
    }
    Ok(value)
}

pub fn resolve(document: &Path, target: &str, local: bool) -> Result<Target, String> {
    let target = target.trim();
    if let Some(anchor) = target.strip_prefix('#') {
        return Ok(Target::Anchor(decode(anchor)?));
    }
    if target.starts_with("//") {
        return resolve(document, &format!("https:{target}"), local);
    }
    let windows_path = local
        && target.as_bytes().get(1) == Some(&b':')
        && target
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && target
            .as_bytes()
            .get(2)
            .is_some_and(|c| *c == b'/' || *c == b'\\');
    if !windows_path && let Ok(url) = url::Url::parse(target) {
        return if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() {
            Ok(Target::Web(url.to_string()))
        } else {
            Err(format!("unsupported link scheme: {}", url.scheme()))
        };
    }
    let (path, fragment) = target
        .split_once('#')
        .map_or((target, None), |(p, f)| (p, Some(f)));
    let path = path.split_once('?').map_or(path, |(p, _)| p);
    let path = decode(path)?;
    let fragment = fragment.map(decode).transpose()?;
    if path.is_empty() {
        return Ok(Target::Anchor(fragment.unwrap_or_default()));
    }
    // A colon in the first relative segment denotes an unsupported or malformed
    // scheme. Do not accidentally pass it to an OS URL/file opener.
    if !windows_path
        && path
            .split(['/', '\\'])
            .next()
            .is_some_and(|p| p.contains(':'))
    {
        return Err("invalid document link".into());
    }
    let path = if local {
        document.parent().unwrap_or(Path::new(".")).join(path)
    } else {
        // Host paths use POSIX separators regardless of the client's platform.
        if path.contains('\\') {
            return Err("remote document paths must use '/'".into());
        }
        if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            let doc = document.to_string_lossy().replace('\\', "/");
            let parent = doc.rsplit_once('/').map_or(".", |(parent, _)| parent);
            PathBuf::from(format!("{parent}/{path}"))
        }
    };
    Ok(Target::File { path, fragment })
}

pub const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Validate the format and dimensions before sending encoded data to GPUI.
pub fn image_format(bytes: &[u8]) -> Result<gpui::ImageFormat, String> {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    if prefix
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with("<svg")
        || (prefix.trim_start().starts_with("<?xml") && prefix.contains("<svg"))
    {
        return Ok(gpui::ImageFormat::Svg);
    }
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| e.to_string())?;
    if u64::from(width) * u64::from(height) > 32_000_000 {
        return Err("image exceeds 32 megapixels".into());
    }
    use gpui::ImageFormat as G;
    use image::ImageFormat as I;
    Ok(match format {
        I::Png => G::Png,
        I::Jpeg => G::Jpeg,
        I::Gif => G::Gif,
        I::WebP => G::Webp,
        I::Bmp => G::Bmp,
        I::Tiff => G::Tiff,
        I::Ico => G::Ico,
        I::Pnm => G::Pnm,
        _ => return Err("unsupported image format".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_links_keep_the_document_directory_and_decode_fragments() {
        assert_eq!(
            resolve(Path::new("/repo/docs/readme.md"), "images/a%20b.png", false).unwrap(),
            Target::File {
                path: "/repo/docs/images/a b.png".into(),
                fragment: None
            }
        );
        assert_eq!(
            resolve(
                Path::new("/repo/docs/readme.md"),
                "../other.md#%E6%A0%87%E9%A2%98",
                false
            )
            .unwrap(),
            Target::File {
                path: "/repo/docs/../other.md".into(),
                fragment: Some("标题".into())
            }
        );
        assert_eq!(
            resolve(Path::new("/repo/readme.md"), "#hello-world-1", true).unwrap(),
            Target::Anchor("hello-world-1".into())
        );
        assert_eq!(
            resolve(Path::new("/repo/readme.md"), "/elsewhere/intro.md", false).unwrap(),
            Target::File {
                path: "/elsewhere/intro.md".into(),
                fragment: None
            }
        );
        assert_eq!(
            resolve(Path::new("docs/readme.md"), "a%23b.md#intro", true).unwrap(),
            Target::File {
                path: Path::new("docs").join("a#b.md"),
                fragment: Some("intro".into())
            }
        );
    }

    #[test]
    fn markdown_links_only_use_the_browser_for_web_urls() {
        let doc = Path::new("/repo/readme.md");
        assert!(matches!(
            resolve(doc, "https://example.com/a#b", false),
            Ok(Target::Web(_))
        ));
        for link in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:image/svg+xml,test",
            "vscode://file/test",
            "bad%00path",
            "C:\\local\\image.png",
        ] {
            assert!(resolve(doc, link, false).is_err(), "{link}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn markdown_windows_relative_paths_are_resolved_on_the_owning_host() {
        assert_eq!(
            resolve(
                Path::new("D:\\project\\docs\\readme.md"),
                "images/picture.png",
                true
            )
            .unwrap(),
            Target::File {
                path: Path::new("D:\\project\\docs").join("images/picture.png"),
                fragment: None
            }
        );
    }
}
