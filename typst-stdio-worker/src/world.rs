use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Datelike;
use flate2::read::GzDecoder;
use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};

use crate::i18n;

fn package_spec(id: &FileId) -> Option<&PackageSpec> {
    match id.root() {
        VirtualRoot::Package(spec) => Some(spec),
        VirtualRoot::Project => None,
    }
}

fn package_label(id: &FileId) -> String {
    package_spec(id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "not package file".to_string())
}

fn file_id(spec: Option<&PackageSpec>, vpath: VirtualPath) -> FileId {
    let root = spec
        .cloned()
        .map(VirtualRoot::Package)
        .unwrap_or(VirtualRoot::Project);
    FileId::new(RootedPath::new(root, vpath))
}

fn main_file_id() -> FileId {
    file_id(
        None,
        VirtualPath::new("main.typ").expect("\"main.typ\" is a valid virtual path"),
    )
}

pub struct TypstBotWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    /// Canonicalized `TYPST_PACKAGE_PATH` / `--package-path`.
    /// Non-existent path skipped at startup.
    canonical_local_package_dir: Option<PathBuf>,
    /// Canonicalized package cache directory (downloads, preview fallback).
    /// `None` if unavailable or not configured.
    canonical_preview_package_dir: Option<PathBuf>,
    allow_download: bool,
}

impl TypstBotWorld {
    /// Create a new world. Returns `(world, font_count)`.
    pub fn new(
        font_paths: &[PathBuf],
        preview_package_dir: Option<PathBuf>,
        local_package_dir: Option<PathBuf>,
        allow_download: bool,
        meter: bool,
    ) -> (Self, usize) {
        let library = LazyHash::new(Library::builder().build());
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        let mut db = fontdb::Database::new();
        if font_paths.is_empty() {
            db.load_system_fonts();
        } else {
            for path in font_paths {
                db.load_fonts_dir(path);
            }
        }

        // Cache file -> Bytes so .ttc / collection files containing many faces
        // are only read once. `Bytes` is internally Arc-like; cloning bumps a
        // refcount without copying the bytes.
        let mut file_cache: HashMap<PathBuf, Bytes> = HashMap::new();
        for face in db.faces() {
            if let fontdb::Source::File(ref path) = face.source {
                let bytes = match file_cache.get(path) {
                    Some(b) => b.clone(),
                    None => {
                        let file = match std::fs::File::open(path) {
                            Ok(f) => f,
                            Err(_) => continue,
                        };
                        // It's safe IF: font files are not modified while the worker is running.
                        // populate() ensures the file is fully mapped into memory, avoiding partial reads.
                        let mmap = unsafe { memmap2::MmapOptions::new()
                            // .populate()
                            .map(&file) }
                            .expect("mmap failed after successful open");
                        let b = Bytes::new(mmap);
                        file_cache.insert(path.clone(), b.clone());
                        b
                    }
                };
                if let Some(font) = Font::new(bytes, face.index) {
                    book.push(font.info().clone());
                    fonts.push(font);
                }
            }
        }
        if meter {
            let font_blob_bytes: u64 = file_cache.values().map(|b| b.len() as u64).sum();
            tracing::info!(
                fonts = fonts.len(),
                font_files = file_cache.len(),
                font_blob_bytes,
                font_blob_human = %crate::util::human_bytes::format_u64(font_blob_bytes),
                "{}", i18n::log_fonts_loaded()
            );
        }
        drop(file_cache);

        let font_count = fonts.len();
        let source = Source::new(main_file_id(), String::new());

        let canonical_package_dir = preview_package_dir.and_then(|dir| {
            if allow_download {
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tracing::warn!(
                        path = %dir.display(),
                        error = %e,
                        "{}", i18n::log_cache_create_failed()
                    );
                    return None;
                }
            }
            match dir.canonicalize() {
                Ok(canon) => Some(canon),
                Err(e) => {
                    tracing::warn!(
                        path = %dir.display(),
                        error = %e,
                        "{}", i18n::log_cache_canonicalize_failed()
                    );
                    None
                }
            }
        });

        let canonical_local_package_dir = local_package_dir.and_then(|dir| {
            if !dir.exists() {
                tracing::debug!(
                    path = %dir.display(),
                    "{}", i18n::log_path_not_exist()
                );
                return None;
            }
            match dir.canonicalize() {
                Ok(canon) => Some(canon),
                Err(e) => {
                    tracing::warn!(
                        path = %dir.display(),
                        error = %e,
                        "{}", i18n::log_path_canonicalize_failed()
                    );
                    None
                }
            }
        });

        let world = Self {
            library,
            book: LazyHash::new(book),
            fonts,
            source,
            canonical_local_package_dir,
            canonical_preview_package_dir: canonical_package_dir,
            allow_download,
        };

        (world, font_count)
    }

    pub fn update_source(&mut self, text: String) {
        self.source = Source::new(main_file_id(), text);
    }

    pub fn main_id(&self) -> FileId {
        self.source.id()
    }

    /// Download a package tarball from the Typst registry and extract it into
    /// the cache directory. Returns the path to the extracted package directory.
    ///
    /// Uses a per-PID temporary directory + atomic `rename` to avoid races when
    /// multiple worker processes download the same package concurrently.
    fn download_package(
        &self,
        spec: &PackageSpec,
        cache_dir: &Path,
    ) -> FileResult<PathBuf> {
        let url = format!(
            "https://packages.typst.org/{}/{}-{}.tar.gz",
            spec.namespace, spec.name, spec.version
        );
        tracing::info!(url = %url, "{}", i18n::log_downloading_package());

        let mut response = ureq::get(&url).call().map_err(|e| {
            FileError::Package(PackageError::NetworkFailed(Some(
                e.to_string().into(),
            )))
        })?;

        if response.status() != 200 {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        }

        let package_dir = cache_dir
            .join(spec.namespace.as_str())
            .join(spec.name.as_str())
            .join(spec.version.to_string());

        // Extract into a PID-namespaced temp directory so concurrent workers
        // never write to the same directory simultaneously.
        let tmp_dir = cache_dir
            .join(spec.namespace.as_str())
            .join(spec.name.as_str())
            .join(format!(".{}.download.{}", spec.version, std::process::id()));

        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).map_err(|e| {
            FileError::Package(PackageError::Other(Some(
                format!("failed to create temp package directory: {e}").into(),
            )))
        })?;

        let reader = response.body_mut().as_reader();
        let gz = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(gz);
        if let Err(e) = archive.unpack(&tmp_dir) {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(FileError::Package(PackageError::Other(Some(
                format!("failed to extract package: {e}").into(),
            ))));
        }

        // Atomic rename: if it succeeds we placed the directory; if it fails
        // with ENOTEMPTY/EEXIST another worker already installed the package.
        match std::fs::rename(&tmp_dir, &package_dir) {
            Ok(()) => {
                tracing::info!(
                    path = %package_dir.display(),
                    "{}", i18n::log_package_downloaded()
                );
            }
            Err(_) if package_dir.exists() => {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                tracing::info!(
                    path = %package_dir.display(),
                    "{}", i18n::log_package_already_exists()
                );
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(FileError::Package(PackageError::Other(Some(
                    format!("failed to install package: {e}").into(),
                ))));
            }
        }

        Ok(package_dir)
    }

    fn package_roots_in_resolution_order<'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a PathBuf> + 'a {
        self.canonical_local_package_dir
            .iter()
            .chain(self.canonical_preview_package_dir.iter())
    }

    fn try_download_registry_package(&self, spec: &PackageSpec) -> FileResult<()> {
        let Some(cache_dir) = self.canonical_preview_package_dir.as_ref() else {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        };
        if spec.namespace.as_str() != "preview" {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        }
        if !self.allow_download {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        }
        self.download_package(spec, cache_dir)?;
        Ok(())
    }

    /// Resolve a package file path on disk with directory traversal protection.
    fn resolve_package_path(
        &self,
        spec: &PackageSpec,
        vpath: &VirtualPath,
    ) -> FileResult<PathBuf> {
        if self.canonical_local_package_dir.is_none() && self.canonical_preview_package_dir.is_none() {
            return Err(FileError::Package(PackageError::Other(Some(
                "no package cache or package path configured".into(),
            ))));
        }

        let relative = Path::new(vpath.get_without_slash());
        // Cheap pre-flight: reject any traversal segment outright. VirtualPath
        // should already normalize but we double-check before touching the FS.
        for component in relative.components() {
            use std::path::Component;
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => return Err(FileError::AccessDenied),
            }
        }

        let mut package_version_dir: Option<PathBuf> = None;
        for root in self.package_roots_in_resolution_order() {
            let candidate = root
                .join(spec.namespace.as_str())
                .join(spec.name.as_str())
                .join(spec.version.to_string());
            if candidate.is_dir() {
                package_version_dir = Some(candidate);
                break;
            }
        }

        if package_version_dir.is_none() {
            self.try_download_registry_package(spec)?;
            if let Some(cache_dir) = &self.canonical_preview_package_dir {
                let candidate = cache_dir
                    .join(spec.namespace.as_str())
                    .join(spec.name.as_str())
                    .join(spec.version.to_string());
                if candidate.is_dir() {
                    package_version_dir = Some(candidate);
                }
            }
        }

        let Some(package_version_dir) = package_version_dir else {
            return Err(FileError::Package(PackageError::NotFound(spec.clone())));
        };

        let package_base = package_version_dir.canonicalize().map_err(|_| {
            FileError::Package(PackageError::NotFound(spec.clone()))
        })?;

        let resolved = package_base.join(relative);
        let canon_file = resolved
            .canonicalize()
            .map_err(|_| FileError::NotFound(relative.into()))?;

        if !canon_file.starts_with(&package_base) {
            return Err(FileError::AccessDenied);
        }

        Ok(canon_file)
    }
}

impl typst::World for TypstBotWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        tracing::debug!(
            name: "typst_world_access",
            package = %package_label(&id),
            file_path = %id.vpath().get_without_slash()
        , "{}", i18n::log_source_access());
        if id == self.source.id() {
            return Ok(self.source.clone());
        }

        if let Some(spec) = package_spec(&id) {
            let path = self.resolve_package_path(spec, id.vpath())?;
            let text = std::fs::read_to_string(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileError::NotFound(path.clone()),
                std::io::ErrorKind::PermissionDenied => FileError::AccessDenied,
                _ => FileError::Other(Some(e.to_string().into())),
            })?;
            return Ok(Source::new(id, text));
        }

        Err(FileError::NotFound(PathBuf::from(id.vpath().get_without_slash())))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        tracing::debug!(
            name: "typst_world_access",
            package = %package_label(&id),
            file_path = %id.vpath().get_without_slash()
        , "{}", i18n::log_file_access());
        if let Some(spec) = package_spec(&id) {
            let path = self.resolve_package_path(spec, id.vpath())?;
            let data = std::fs::read(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => FileError::NotFound(path.clone()),
                std::io::ErrorKind::PermissionDenied => FileError::AccessDenied,
                _ => FileError::Other(Some(e.to_string().into())),
            })?;
            return Ok(Bytes::new(data));
        }

        Err(FileError::NotFound(PathBuf::from(id.vpath().get_without_slash())))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let date = if let Some(offset) = offset {
            let adjusted = chrono::Utc::now()
                + chrono::Duration::milliseconds((offset.seconds() * 1000.0) as i64);
            adjusted.date_naive()
        } else {
            chrono::Local::now().date_naive()
        };

        Datetime::from_ymd(date.year(), date.month() as u8, date.day() as u8)
    }
}
