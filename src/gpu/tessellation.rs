use color::Colorer;
use errors::Result;
use geom::{Ellipse, Point, Poly};
use gpu::{GpuNormal, GpuVertex};
use lyon::path_iterator::math::Vec2;
use lyon::tessellation::*;
use lyon::tessellation::geometry_builder::{VertexBuffers, simple_builder};
use mesh::DrawMode;

const ELLIPSE_TOLERANCE: f32 = 0.00001;

#[derive(Debug, Default)]
pub struct Tessellation {
    pub vertices: Vec<GpuVertex>,
    pub normals: Vec<GpuNormal>,
    pub indices: Vec<u32>,
}

impl Tessellation {
    fn from_fill_buffer(buffer: VertexBuffers<FillVertex>, colorer: Colorer) -> Self {
        let mut tessellation = Tessellation::default();
        for v in buffer.vertices {
            let point = Point::from(v.position);
            let color = colorer.color(point);
            tessellation
                .vertices
                .push(GpuVertex::from((point, color)));
            tessellation
                .normals
                .push(GpuNormal { normal: (v.normal.x, v.normal.y, 0.0) });
        }
        tessellation.indices = buffer.indices.into_iter().map(Into::into).collect();
        tessellation
    }
    fn from_stroke_buffer(buffer: VertexBuffers<StrokeVertex>, colorer: Colorer) -> Self {
        let mut tessellation = Tessellation::default();
        for v in buffer.vertices {
            let point = Point::from(v.position);
            let color = colorer.color(point);
            tessellation
                .vertices
                .push(GpuVertex::from((point, color)));
            tessellation
                .normals
                .push(GpuNormal { normal: (v.normal.x, v.normal.y, 0.0) });
        }
        tessellation.indices = buffer.indices.into_iter().map(Into::into).collect();
        tessellation
    }
}

pub trait Tessellate {
    fn tessellate_fill(&self, colorer: Colorer) -> Result<Tessellation>;
    fn tessellate_stroke(&self, thickness: f32, colorer: Colorer) -> Result<Tessellation>;
}

impl Tessellate for Ellipse {
    fn tessellate_fill(&self, colorer: Colorer) -> Result<Tessellation> {
        let mut vertex_buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
        match self.height {
            Some(height) => {
                basic_shapes::fill_ellipse(self.center.into(),
                                           Vec2::new(self.width, height),
                                           self.rotation,
                                           self.tolerance.unwrap_or(ELLIPSE_TOLERANCE),
                                           &mut simple_builder(&mut vertex_buffers));
            }
            None => {
                basic_shapes::fill_circle(self.center.into(),
                                          self.width,
                                          self.tolerance.unwrap_or(ELLIPSE_TOLERANCE),
                                          &mut simple_builder(&mut vertex_buffers));
            }
        };
        Ok(Tessellation::from_fill_buffer(vertex_buffers, colorer))
    }
    fn tessellate_stroke(&self, thickness: f32, colorer: Colorer) -> Result<Tessellation> {
        let mut vertex_buffers: VertexBuffers<StrokeVertex> = VertexBuffers::new();
        match self.height {
            Some(height) => {
                basic_shapes::stroke_ellipse(self.center.into(),
                                             Vec2::new(self.width, height),
                                             self.rotation,
                                            &StrokeOptions::default()
                                                 .with_line_width(thickness)
                                                 .with_tolerance(self.tolerance
                                                                     .unwrap_or(ELLIPSE_TOLERANCE)),
                                             &mut simple_builder(&mut vertex_buffers));
            }
            None => {
                basic_shapes::stroke_circle(self.center.into(),
                                            self.width,
                                            &StrokeOptions::default()
                                                 .with_line_width(thickness)
                                                 .with_tolerance(self.tolerance
                                                                     .unwrap_or(ELLIPSE_TOLERANCE)),
                                            &mut simple_builder(&mut vertex_buffers));
            }
        };
        Ok(Tessellation::from_stroke_buffer(vertex_buffers, colorer))
    }
}

impl<P: Poly> Tessellate for P {
    fn tessellate_fill(&self, colorer: Colorer) -> Result<Tessellation> {
        let mut vertex_buffers: VertexBuffers<FillVertex> = VertexBuffers::new();
        basic_shapes::fill_polyline(self.vertices().into_iter().map(Into::into),
                                    &mut FillTessellator::new(),
                                    &FillOptions::default(),
                                    &mut simple_builder(&mut vertex_buffers))?;
        Ok(Tessellation::from_fill_buffer(vertex_buffers, colorer))
    }
    fn tessellate_stroke(&self, thickness: f32, colorer: Colorer) -> Result<Tessellation> {
        let mut vertex_buffers: VertexBuffers<StrokeVertex> = VertexBuffers::new();
        basic_shapes::stroke_polyline(self.vertices().into_iter().map(Into::into),
                                      true,
                                      &StrokeOptions::default(),
                                      &mut simple_builder(&mut vertex_buffers));
        Ok(Tessellation::from_stroke_buffer(vertex_buffers, colorer))
    }
}