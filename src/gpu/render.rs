use gpu::{Factory, Gpu, GpuMesh};
use std::rc::Rc;
use composition::{Composition, Layer};
use gpu::shaders::{GpuShader, Shader};
use mesh::Mesh;
use glium::Surface;
use glium::texture::{MipmapsOption, Texture2d, UncompressedFloatFormat};
use glium::uniforms::MagnifySamplerFilter;
use errors::Result;
use poly::Rect;
use gpu::programs::Library;

pub struct GpuLayer {
    src: Mesh,
    shader: GpuShader,
    cached_mesh: GpuMesh,
}

impl Factory<Layer> for GpuLayer {
    fn produce(spec: Layer, gpu: Rc<Gpu>) -> Result<GpuLayer> {
        let (shader, mesh) = match spec {
            Layer::Mesh(mesh) => (Shader::Default, mesh),
            Layer::ShadedMesh { shader, mesh } => (shader, mesh),
        };
        Ok(GpuLayer {
            shader: GpuShader::produce(shader, gpu.clone())?,
            cached_mesh: GpuMesh::produce(mesh.clone(), gpu.clone())?,
            src: mesh,
        })
    }
}

impl GpuLayer {
    pub fn step(mut self, frame: usize) -> Result<Self> {
        self.cached_mesh.scale = self.src.scale.tween(frame);
        Ok(self)
    }
    pub fn render<'a>(&'a self) -> (&'a GpuShader, &'a GpuMesh) {
        (&self.shader, &self.cached_mesh)
    }
}

pub struct DrawCtx<'a, 'b> {
    frame: usize,
    library: &'b Library,
    cmds: Vec<(&'a GpuShader, &'a GpuMesh)>,
}

struct BufferSpec {
    pub width: u32,
    pub height: u32,
}

struct Buffer {
    targets: [Rc<Texture2d>; 2],
    blitter: (GpuShader, GpuMesh),
}

impl Buffer {
    pub fn blitter<'a>(&'a self) -> Vec<(&'a GpuShader, &'a GpuMesh)> {
        vec![(&self.blitter.0, &self.blitter.1)]
    }

    /// Draws commands to the buffer and returns a set of commands to draw this
    /// buffer to screen.
    pub fn draw<'a>(&'a self, ctx: DrawCtx) -> Result<Vec<(&'a GpuShader, &'a GpuMesh)>> {
        let mut surfaces = [self.targets[0].as_surface(), self.targets[1].as_surface()];
        for (ref shader, ref mesh) in ctx.cmds.into_iter() {
            shader.draw(
                ctx.library,
                ctx.frame,
                &mut surfaces[0],
                mesh,
                Some(self.targets[1].as_ref()),
            )?;
            surfaces[0].fill(&surfaces[1], MagnifySamplerFilter::Linear);
        }
        Ok(self.blitter())
    }

    pub fn front(&self) -> Rc<Texture2d> {
        self.targets[0].clone()
    }
}

impl Factory<BufferSpec> for Buffer {
    fn produce(spec: BufferSpec, gpu: Rc<Gpu>) -> Result<Self> {
        let target = || -> Result<Texture2d> {
            Texture2d::empty_with_format(
                gpu.as_ref(),
                UncompressedFloatFormat::F32F32F32F32,
                MipmapsOption::AutoGeneratedMipmaps,
                spec.width,
                spec.height,
            ).map_err(Into::into)
        };

        let targets = [Rc::new(target()?), Rc::new(target()?)];
        for target in targets.iter() {
            target.as_ref().as_surface().clear_color(0.0, 0.0, 0.0, 0.0)
        }
        let blitter = (
            GpuShader::Texture(targets[0].clone()),
            GpuMesh::produce(Mesh::from(Rect::frame()), gpu)?,
        );
        Ok(Self { targets, blitter })
    }
}

pub struct RenderSpec {
    pub width: u32,
    pub height: u32,
    pub composition: Composition,
}

pub struct Render {
    layers: Vec<GpuLayer>,
    buffer: Buffer,
}

impl Factory<RenderSpec> for Render {
    fn produce(spec: RenderSpec, gpu: Rc<Gpu>) -> Result<Render> {
        Ok(Render {
            layers: spec.composition
                .layers()
                .into_iter()
                .map(|l| GpuLayer::produce(l, gpu.clone()))
                .collect::<Result<Vec<GpuLayer>>>()?,
            buffer: Buffer::produce(
                BufferSpec {
                    width: spec.width,
                    height: spec.height,
                },
                gpu,
            )?,
        })
    }
}

impl Render {
    pub fn step(self, frame: usize) -> Result<Self> {
        Ok(Self {
            layers: self.layers
                .into_iter()
                .map(|l| l.step(frame))
                .collect::<Result<Vec<GpuLayer>>>()?,
            ..self
        })
    }

    pub fn render<'a>(
        &'a self,
        library: &Library,
        frame: usize,
    ) -> Result<Vec<(&'a GpuShader, &'a GpuMesh)>> {
        self.buffer.draw(DrawCtx {
            library,
            frame,
            cmds: self.cmds(),
        })
    }

    pub fn buffer(&self) -> Rc<Texture2d> {
        self.buffer.front()
    }

    fn cmds<'a>(&'a self) -> Vec<(&'a GpuShader, &'a GpuMesh)> {
        self.layers.iter().map(|l| l.render()).collect()
    }
}
