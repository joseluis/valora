use color::Colorer;
use gpu::Shader;
use mesh::Mesh;
use poly::Rect;
use std::mem::swap;
use std::rc::Rc;

pub enum Layer {
    Mesh(Mesh),
    ShadedMesh { shader: Shader, mesh: Mesh },
}

impl Layer {
    pub fn once<L: Into<LayerInput>>(src: L, render_frame: usize) -> Vec<Layer> {
        src.into()
            .map(|layer| Layer::freeze_frame(layer, render_frame))
            .collect()
    }

    fn freeze_frame(src: Layer, render_frame: usize) -> Layer {
        let wrap_shader = |shader| Shader::Intermittent {
            src: Rc::new(shader),
            predicate: Rc::new(move |current_frame| current_frame == render_frame),
        };
        match src {
            Layer::Mesh(mesh) => Layer::ShadedMesh {
                shader: wrap_shader(Shader::Default),
                mesh,
            },
            Layer::ShadedMesh { shader, mesh } => Layer::ShadedMesh {
                shader: wrap_shader(shader),
                mesh,
            },
        }
    }
}

impl From<Mesh> for Layer {
    fn from(mesh: Mesh) -> Self {
        Layer::Mesh(mesh)
    }
}

impl<S: Into<Shader>> From<S> for Layer {
    fn from(shader_src: S) -> Self {
        Layer::ShadedMesh {
            shader: shader_src.into(),
            mesh: Mesh::from(Rect::frame()),
        }
    }
}

impl<S: Into<Shader>> From<(S, Mesh)> for Layer {
    fn from((shader, mesh): (S, Mesh)) -> Self {
        Layer::ShadedMesh {
            shader: shader.into(),
            mesh,
        }
    }
}

pub enum LayerInput {
    Single(Layer),
    Many(Vec<Layer>),
}

impl<T: Into<Layer>> From<T> for LayerInput {
    fn from(t: T) -> Self {
        LayerInput::Single(t.into())
    }
}

impl<T: Into<Layer>> From<Vec<T>> for LayerInput {
    fn from(ts: Vec<T>) -> Self {
        LayerInput::Many(ts.into_iter().map(Into::into).collect())
    }
}

impl Iterator for LayerInput {
    type Item = Layer;
    fn next(&mut self) -> Option<Layer> {
        let mut output = None;
        let mut tmp = LayerInput::Many(Vec::new());
        swap(self, &mut tmp);
        *self = match tmp {
            LayerInput::Single(il) => {
                output = Some(il);
                LayerInput::Many(Vec::new())
            }
            LayerInput::Many(mut ts) => {
                output = ts.pop();
                LayerInput::Many(ts)
            }
        };
        output
    }
}

#[derive(Default)]
pub struct Composition {
    layers: Vec<Layer>,
}

impl Composition {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn solid_layer(self, colorer: Colorer) -> Self {
        self.add(Mesh::from(Rect::frame()).with_colorer(colorer))
    }

    pub fn add<L: Into<LayerInput>>(mut self, layer: L) -> Self {
        self.layers.extend(layer.into());
        self
    }

    pub fn layers(self) -> Vec<Layer> {
        self.layers
    }
}
