use anyhow::Result;
use klystron::{
    runtime_2d::{event::WindowEvent, launch, App2D},
    DrawType, Engine, FramePacket, Matrix4, Object, Vertex, WinitBackend, UNLIT_FRAG, UNLIT_VERT,
};
use shaderpark::{MaterialAutoUpdate, print_result};

struct MyApp {
    auto_update: MaterialAutoUpdate,
    quad: Object,
}

impl App2D for MyApp {
    const TITLE: &'static str = "2D example app";
    type Args = ();

    fn new(engine: &mut WinitBackend, _args: Self::Args) -> Result<Self> {
        let material = engine.add_material(UNLIT_VERT, UNLIT_FRAG, DrawType::Triangles)?;

        let (vertices, indices) = fullscreen_quad();
        let mesh = engine.add_mesh(&vertices, &indices)?;

        let mut auto_update = MaterialAutoUpdate::new("./shaders", engine, DrawType::Triangles, None)?;
        auto_update.manual_update("./shaders/fullscreen.vert")?;
        auto_update.manual_update("./shaders/unlit.frag")?;

        let quad = Object {
            mesh,
            transform: Matrix4::identity(),
            material,
        };

        Ok(Self {
            auto_update,
            quad,
        })
    }

    fn event(&mut self, _event: &WindowEvent, _engine: &mut WinitBackend) -> Result<()> {
        Ok(())
    }

    fn frame(&mut self, engine: &mut WinitBackend) -> FramePacket {
        print_result(self.auto_update.update(engine));
        self.quad.material = self.auto_update.material();
        FramePacket {
            objects: vec![self.quad],
        }
    }
}

fn fullscreen_quad() -> (Vec<Vertex>, Vec<u16>) {
    let vertices = vec![
        Vertex::new([-1.0, -1.0, 0.0], [0.; 3]),
        Vertex::new([-1.0, 1.0, 0.0], [1.; 3]),
        Vertex::new([1.0, -1.0, 0.0], [1.; 3]),
        Vertex::new([1.0, 1.0, 0.0], [1.; 3]),
    ];

    //let indices = vec![2, 1, 0, 3, 1, 2];
    let indices = vec![0, 1, 2, 2, 1, 3];

    (vertices, indices)
}


fn main() -> Result<()> {
    launch::<MyApp>(())
}
