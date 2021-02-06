use anyhow::{format_err, Context, Result};
use klystron::{
    runtime_3d::{launch, App},
    DrawType, Engine, FramePacket, Material, Mesh, Object, Vertex, UNLIT_FRAG, UNLIT_VERT,
};
use nalgebra::{Matrix4, Vector3, Vector4};
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use shaderc::{Compiler, ShaderKind};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

pub struct MaterialAutoUpdate {
    _file_watcher: RecommendedWatcher,
    file_watch_rx: Receiver<DebouncedEvent>,
    file_watch_tx: Sender<DebouncedEvent>,
    material: Material,
    compiler: Compiler,
    vert: Vec<u8>,
    frag: Vec<u8>,
}

impl MaterialAutoUpdate {
    pub fn new(shader_dir: impl AsRef<Path>, engine: &mut dyn Engine) -> Result<Self> {
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
        })
    }

    pub fn manual_update(&mut self, path: impl AsRef<Path>) -> Result<()> {
        Ok(self.file_watch_tx.send(DebouncedEvent::Write(path.as_ref().into()))?)
    }

    pub fn material(&self) -> Material {
        self.material
    }

    pub fn update(&mut self, engine: &mut dyn Engine) {
        match self.file_watch_rx.try_recv() {
            Ok(DebouncedEvent::Create(p)) | Ok(DebouncedEvent::Write(p)) => {
                if let Err(e) = self.update_shader(&p, engine) {
                    println!("Shader compilation error: {:?}", e);
                }
            }
            _ => (),
        }
    }

    fn update_shader(&mut self, path: &Path, engine: &mut dyn Engine) -> Result<()> {
        let kind = match path.extension().and_then(|v| v.to_str()) {
            Some("vert") => ShaderKind::Vertex,
            Some("frag") => ShaderKind::Fragment,
            None | Some(_) => return Ok(()),
        };

        if path.file_stem().unwrap() != "unlit" {
            return Ok(());
        }

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

        println!("Successfully loaded {:?} shader: {:?}", kind, path);

        Ok(())
    }
}
