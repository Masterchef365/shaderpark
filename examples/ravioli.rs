use anyhow::Result;
use klystron::{
    runtime_3d::{launch, App},
    Engine, FramePacket, Mesh, Object, Vertex, DrawType
};
use nalgebra::{Matrix4, Vector3, Vector4};
use shaderpark::{MaterialAutoUpdate, print_result};

struct MyApp {
    auto_update: MaterialAutoUpdate,
    mesh: Mesh,
    time: f32,
}

impl App for MyApp {
    const NAME: &'static str = "Ravioli";

    type Args = ();


    fn new(engine: &mut dyn Engine, _args: Self::Args) -> Result<Self> {
        let (vertices, indices) = ravioli(1., 1.8, 1.6, 30);
        let mesh = engine.add_mesh(&vertices, &indices)?;
        let mut auto_update = MaterialAutoUpdate::new("./shaders", engine, DrawType::Triangles, None)?;
        auto_update.manual_update("./shaders/unlit.frag")?;
        auto_update.manual_update("./shaders/unlit.vert")?;

        Ok(Self {
            mesh,
            time: 0.,
            auto_update,
        })
    }

    fn next_frame(&mut self, engine: &mut dyn Engine) -> Result<FramePacket> {
        print_result(self.auto_update.update(engine));

        //let scale = 200.;
        let scale = 1.;
        let trans = Matrix4::new_translation(&Vector3::new(1., 1., -1.))
            * Matrix4::from_diagonal(&Vector4::new(scale, scale, scale, 1.));

        let top = Object {
            material: self.auto_update.material(),
            mesh: self.mesh,
            transform: trans * Matrix4::identity(),
        };
        let bottom = Object {
            material: self.auto_update.material(),
            mesh: self.mesh,
            transform: trans * Matrix4::from_diagonal(&Vector4::new(1., -1., 1., 1.)),
        };

        engine.update_time_value(self.time)?;
        self.time += 0.01;
        Ok(FramePacket {
            objects: vec![top, bottom],
        })
    }
}

fn main() -> Result<()> {
    let vr = std::env::args().skip(1).next().is_some();
    launch::<MyApp>(vr, ())
}

fn ravioli(width: f32, radius: f32, offset: f32, steps: usize) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::with_capacity(steps * steps);
    let mut indices = Vec::with_capacity(vertices.len() * 2 * 3);

    for x in 0..steps {
        let x = x as f32 / steps as f32;
        for z in 0..steps {
            let z = z as f32 / steps as f32;
            let x = x * 2. - 1.;
            let z = z * 2. - 1.;

            let r = x * x + z * z;
            let height = (radius * radius - r).sqrt();
            vertices.push(Vertex {
                pos: [x * width, height - offset, z * width],
                //color: [x, z, 0.],
                color: [x, z, height - offset],
            });
        }
    }

    for i in 0..vertices.len() - steps {
        if i % steps == steps - 1 {
            continue;
        };

        let i = i as u16;
        let tl = i + 0;
        let tr = i + 1;
        let bl = i + 0 + steps as u16;
        let br = i + 1 + steps as u16;

        // Outside
        indices.push(tl);
        indices.push(tr);
        indices.push(bl);

        indices.push(bl);
        indices.push(tr);
        indices.push(br);

        // Inside
        indices.push(bl);
        indices.push(tr);
        indices.push(tl);

        indices.push(br);
        indices.push(tr);
        indices.push(bl);
    }

    (vertices, indices)
}
