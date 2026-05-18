use engine_graphics::{GraphicsError, GraphicsResult};
use engine_render::Mesh;
use graphics_wgpu::{wgpu, WgpuGraphics};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GpuVertex {
    position: [f32; 3],
    color: [f32; 4],
    normal: [f32; 3],
    uv: [f32; 2],
    uv1: [f32; 2],
    tangent: [f32; 4],
}

unsafe impl bytemuck::Zeroable for GpuVertex {}
unsafe impl bytemuck::Pod for GpuVertex {}

impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x4,
        2 => Float32x3,
        3 => Float32x2,
        4 => Float32x2,
        5 => Float32x4
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct WgpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
    index_count: u32,
}

impl WgpuMesh {
    pub fn from_mesh(graphics: &WgpuGraphics, mesh: &Mesh) -> GraphicsResult<Self> {
        if mesh.is_empty() {
            return Err(GraphicsError::InvalidResource(
                "mesh must contain at least one vertex".to_owned(),
            ));
        }

        let vertex_count = u32::try_from(mesh.vertex_count()).map_err(|_| {
            GraphicsError::InvalidResource("mesh has more than u32::MAX vertices".to_owned())
        })?;
        let index_count = u32::try_from(mesh.index_count()).map_err(|_| {
            GraphicsError::InvalidResource("mesh has more than u32::MAX indices".to_owned())
        })?;

        if let Some(index) = mesh
            .indices()
            .iter()
            .copied()
            .find(|index| *index as usize >= mesh.vertex_count())
        {
            return Err(GraphicsError::InvalidResource(format!(
                "mesh index {index} is outside the vertex range"
            )));
        }

        let vertices = mesh
            .vertices()
            .iter()
            .map(|vertex| GpuVertex {
                position: vertex.position,
                color: [
                    vertex.color[0],
                    vertex.color[1],
                    vertex.color[2],
                    vertex.alpha,
                ],
                normal: vertex.normal,
                uv: vertex.uv,
                uv1: vertex.uv1,
                tangent: vertex.tangent,
            })
            .collect::<Vec<_>>();
        let vertex_bytes = bytemuck::cast_slice(&vertices);
        let vertex_buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Mesh Vertex Buffer"),
            size: vertex_bytes.len() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        graphics
            .queue()
            .write_buffer(&vertex_buffer, 0, vertex_bytes);

        let index_buffer = if mesh.is_indexed() {
            let index_bytes = bytemuck::cast_slice(mesh.indices());
            let buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("Neo Mesh Index Buffer"),
                size: index_bytes.len() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            graphics.queue().write_buffer(&buffer, 0, index_bytes);
            Some(buffer)
        } else {
            None
        };

        Ok(Self {
            vertex_buffer,
            index_buffer,
            vertex_count,
            index_count,
        })
    }

    pub(crate) fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        GpuVertex::layout()
    }

    pub(crate) fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub(crate) fn index_buffer(&self) -> Option<&wgpu::Buffer> {
        self.index_buffer.as_ref()
    }

    pub(crate) fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub(crate) fn index_count(&self) -> u32 {
        self.index_count
    }
}
