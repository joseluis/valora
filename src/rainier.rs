//! Tools for composing generative fine art.

use crate::amicola::*;

pub use crate::amicola::{Glsl, Polygon, Shader, UniformBuffer, V2, V4};
pub use rand::{self, rngs::StdRng, Rng, SeedableRng};

use derive_more::DebugCustom;
use failure::{Error, Fail};
use glium::{backend::glutin::headless::Headless, program::Program};
use std::{convert::TryFrom, path::PathBuf, rc::Rc};
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Error>;

#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "valora")]
pub struct Options {
    /// Seed for the rng.
    #[structopt(short = "e", long = "seed", default_value = "0")]
    pub seed: u64,

    /// Width of view pane.
    #[structopt(short = "w", long = "width", default_value = "1024")]
    pub width: u32,

    /// Height of view pane.
    #[structopt(short = "h", long = "height", default_value = "512")]
    pub height: u32,

    /// Scale of view pane.
    #[structopt(short = "s", long = "scale", default_value = "1.0")]
    pub scale: f32,

    /// Frame range to render from the generate scene.
    #[structopt(short = "f", long = "frames", default_value = "1")]
    pub frames: usize,

    /// Prefix of output path. Output is <prefix>/<seed>/<frame_number>.png
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    pub output: PathBuf,
}

pub struct Context<S> {
    pub width: f32,
    pub height: f32,
    pub seed: u64,
    pub frames: usize,
    pub frame: usize,
    pub state: S,
    gpu_ctx: Rc<Headless>,
}

impl<S> From<(Options, S, Rc<Headless>)> for Context<S> {
    fn from((options, state, gpu_ctx): (Options, S, Rc<Headless>)) -> Self {
        Self {
            seed: options.seed,
            width: options.width as f32,
            height: options.height as f32,
            frames: options.frames,
            frame: 0,
            state,
            gpu_ctx,
        }
    }
}

impl<S> Context<S> {
    pub fn normalize(&self, p: V2) -> V2 { V2::new(p.x / self.width, p.y / self.height) }

    pub fn center(&self) -> V2 { V2::new(self.width / 2.0, self.height / 2.0) }

    pub fn full_frame(&self) -> Polygon {
        Polygon::try_from(vec![
            V2::new(0.0, 0.0),
            V2::new(self.width, 0.0),
            V2::new(self.width, self.height),
            V2::new(0.0, self.height),
        ])
        .unwrap()
    }

    pub fn build_shader(&self, source: &str) -> Result<Rc<Program>> {
        Ok(
            Program::from_source(self.gpu_ctx.as_ref(), VERTEX_SHADER, source, None)
                .map(|p| Rc::new(p))?,
        )
    }
}

pub trait Generate<S> {
    type Output;

    fn generate(&self, ctx: &Context<S>, rng: &mut StdRng) -> Self::Output;
}

impl<S, T, F: Fn(&Context<S>, &mut StdRng) -> T> Generate<S> for F {
    type Output = T;

    fn generate(&self, ctx: &Context<S>, rng: &mut StdRng) -> Self::Output { (self)(ctx, rng) }
}

pub trait Render {
    fn render(&self, comp: &mut Sketch);
}

pub trait Composer<S> {
    fn init(rng: &mut StdRng) -> S;
    fn draw(&mut self, ctx: &Context<S>, rng: &mut StdRng, comp: &mut Sketch) -> S;
}

pub trait Sketch {
    fn move_to(&mut self, dest: V2);

    fn line_to(&mut self, dest: V2);

    fn set_color(&mut self, color: V4);

    fn set_shader(&mut self, shader: Shader);

    fn fill(&mut self);
}

pub fn run<S, C: Composer<S>>(mut composer: C) {
    let options = Options::from_args();

    let mut rng = StdRng::seed_from_u64(options.seed);
    let state = <C as Composer<S>>::init(&mut rng);
    let gpu_target = GpuTarget::with_dimensions(options.width as u32, options.height as u32);
    let mut ctx = Context::from((options.clone(), state, gpu_target.ctx()));

    let mut comp = Rainier::new(gpu_target, options.scale);
    let mut frame = 0;

    for i in 0..(options.frames) {
        comp.target.clear();
        ctx.frame = frame % options.frames;
        let next_state = composer.draw(&ctx, &mut rng, &mut comp);
        ctx.state = next_state;
        comp.target.flush();

        let mut save_path = options.output.clone();
        save_path.push(format!("{}", ctx.seed));
        std::fs::create_dir_all(&save_path)
            .expect(&format!("To create save directory {:?}", save_path));
        save_path.push(format!("{}.png", ctx.frame));
        comp.target
            .image()
            .save(save_path)
            .expect("To save output.");
    }
}

pub struct Rainier<T> {
    target: T,
    current_path: Vec<V2>,
    current_shader: Shader,
    current_color: V4,
    scale: f32,
}

impl<T> Rainier<T> {
    fn new(target: T, scale: f32) -> Self {
        Self {
            target,
            current_path: vec![],
            current_shader: Shader::Solid,
            current_color: V4::new(1.0, 1.0, 1.0, 1.0),
            scale,
        }
    }
}

impl<T: RasterTarget> Sketch for Rainier<T> {
    fn move_to(&mut self, dest: V2) { self.current_path = vec![dest * self.scale]; }

    fn line_to(&mut self, dest: V2) { self.current_path.push(dest * self.scale); }

    fn set_color(&mut self, color: V4) { self.current_color = color; }

    fn set_shader(&mut self, shader: Shader) { self.current_shader = shader; }

    fn fill(&mut self) {
        let mut path = vec![];
        std::mem::swap(&mut self.current_path, &mut path);
        self.target.raster(Element {
            path,
            color: self.current_color,
            shader: &self.current_shader,
            raster_method: RasterMethod::Fill,
        });
    }
}
