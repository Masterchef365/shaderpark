use anyhow::{format_err, Context, Result};
use klystron::{DrawType, Engine, Material, UNLIT_FRAG, UNLIT_VERT};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use shaderc::{Compiler, ShaderKind};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// Material update tracker
pub struct MaterialAutoUpdate {
    _file_watcher: RecommendedWatcher,
    file_watch_rx: Receiver<DebouncedEvent>,
    file_watch_tx: Sender<DebouncedEvent>,
    material: Material,
    compiler: Compiler,
    vert: Vec<u8>,
    frag: Vec<u8>,
    prefix: Option<String>,
}

impl MaterialAutoUpdate {
    /// Create a new Material update tracker; will initialize using default materials.
    /// Filters by prefix, if specified
    pub fn new(
        shader_dir: impl AsRef<Path>,
        engine: &mut dyn Engine,
        prefix: Option<String>,
    ) -> Result<Self> {
        let compiler = Compiler::new().context("Shaderc failed to create compiler")?;

        let (file_watch_tx, file_watch_rx) = channel();
        let mut file_watcher = watcher(file_watch_tx.clone(), Duration::from_millis(500))?;
        file_watcher.watch(shader_dir, RecursiveMode::NonRecursive)?;
        let material = engine.add_material(UNLIT_VERT, UNLIT_FRAG, DrawType::Triangles)?;

        Ok(Self {
            compiler,
            vert: UNLIT_VERT.to_vec(),
            frag: UNLIT_FRAG.to_vec(),
            _file_watcher: file_watcher,
            file_watch_rx,
            file_watch_tx,
            material,
            prefix,
        })
    }

    /// Manually update a shader. Decides type based on file extension
    pub fn manual_update(&mut self, path: impl AsRef<Path>) -> Result<()> {
        Ok(self
            .file_watch_tx
            .send(DebouncedEvent::Write(path.as_ref().into()))?)
    }

    pub fn material(&self) -> Material {
        self.material
    }

    /// Poll for a new shader update, and act accordingly
    pub fn update(&mut self, engine: &mut dyn Engine) -> Result<Option<String>> {
        match self.file_watch_rx.try_recv() {
            Ok(DebouncedEvent::Create(p)) | Ok(DebouncedEvent::Write(p)) => {
                self.update_shader(&p, engine)
            }
            _ => Ok(None),
        }
    }

    /// Internal method used to update material
    fn update_shader(&mut self, path: &Path, engine: &mut dyn Engine) -> Result<Option<String>> {
        if let Some(prefix) = self.prefix.as_ref() {
            let has_prefix = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with(prefix))
                .unwrap_or(false);
            if !has_prefix {
                return Ok(None);
            }
        }

        let kind = match path.extension().and_then(|v| v.to_str()) {
            Some("vert") => ShaderKind::Vertex,
            Some("frag") => ShaderKind::Fragment,
            None | Some(_) => return Ok(None),
        };

        let source = fs::read_to_string(path)
            .with_context(|| format_err!("File error loading {:?}", path))?;

        let spv = self
            .compiler
            .compile_into_spirv(&source, kind, path.to_str().unwrap(), "main", None)
            .context("Failed to compile shader")?;
        let spv = spv.as_binary_u8().to_vec();

        if kind == ShaderKind::Vertex {
            self.vert = spv;
        } else {
            self.frag = spv;
        }

        engine.remove_material(self.material)?;
        self.material = engine.add_material(&self.vert, &self.frag, DrawType::Triangles)?;

        Ok(Some(format!(
            "Successfully loaded {:?} shader: {:?}",
            kind, path
        )))
    }
}
