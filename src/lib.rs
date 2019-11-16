#![forbid(unsafe_code)]

mod fps;
mod render;
mod sandpile;

use render::WORLD_SIZE;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    web_sys::window()
        .unwrap_throw()
        .request_animation_frame(&State::new().0.borrow().animation_frame_closure)
        .unwrap_throw();
}

#[derive(Clone)]
struct State(Rc<RefCell<Model>>);

struct Model {
    animation_frame_closure: js_sys::Function,
    keys: HashSet<String>,
    fps: Option<fps::FrameCounter>,
    vr_status: VrStatus,
    renderer: render::Renderer,

    window: web_sys::Window,
    document: web_sys::Document,
    canvas: web_sys::HtmlCanvasElement,
    info_box: web_sys::HtmlParagraphElement,
    brightness_slider: web_sys::HtmlInputElement,
    opacity_slider: web_sys::HtmlInputElement,
    color_sliders: [[web_sys::HtmlInputElement; 3]; 5],

    camera: nalgebra::Isometry3<f32>,
    world: sandpile::World,
}

enum Msg {
    Click,
    MouseMove([i32; 2]),
    KeyDown(String),
    KeyUp(String),

    GotVRDisplays(js_sys::Array),
    DisplayPresenting(web_sys::VrDisplay),
}

enum VrStatus {
    Searching,
    NotSupported,
    NotFound,
    Known(web_sys::VrDisplay),
    RequestedPresentation(web_sys::VrDisplay),
    Presenting(web_sys::VrDisplay),
}

impl State {
    fn new() -> Self {
        let out = Self(Rc::new(RefCell::new(Model::new())));

        {
            let model: &mut Model = &mut out.0.borrow_mut();

            let navigator: web_sys::Navigator = model.window.navigator();
            if js_sys::Reflect::has(&navigator, &"getVRDisplays".into()).unwrap_throw() {
                let state = out.clone();
                let closure = Closure::once(move |vr_displays| {
                    state.update(Msg::GotVRDisplays(js_sys::Array::from(&vr_displays)));
                });
                navigator.get_vr_displays().unwrap_throw().then(&closure);
                closure.forget();
            } else {
                web_sys::console::error_1(
                    &"WebVR is not supported by this browser, on this computer.".into(),
                );

                model.vr_status = VrStatus::NotSupported;
            }

            out.event_listener(&model.canvas, "mousedown", |_| Msg::Click);
            out.event_listener(&model.canvas, "mousemove", |evt| {
                let evt = evt.dyn_into::<web_sys::MouseEvent>().unwrap_throw();
                Msg::MouseMove([evt.movement_x(), evt.movement_y()])
            });
            out.event_listener(&model.document, "keydown", |evt| {
                let evt = evt.dyn_into::<web_sys::KeyboardEvent>().unwrap_throw();
                Msg::KeyDown(evt.key())
            });
            out.event_listener(&model.document, "keyup", |evt| {
                let evt = evt.dyn_into::<web_sys::KeyboardEvent>().unwrap_throw();
                Msg::KeyUp(evt.key())
            });

            let state = out.clone();
            let closure: Closure<dyn FnMut(f64)> = Closure::wrap(Box::new(move |timestamp| {
                state.frame(timestamp);
            }));
            model.animation_frame_closure =
                closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
            closure.forget();
        }

        out
    }

    fn update(&self, msg: Msg) {
        let model: &mut Model = &mut self.0.borrow_mut();

        match msg {
            Msg::Click => {
                if model.document.pointer_lock_element().is_none() {
                    model.canvas.request_pointer_lock();
                }
                if let VrStatus::Known(display) = &model.vr_status {
                    let mut layer = web_sys::VrLayer::new();
                    layer.source(Some(&model.canvas));
                    let layers = js_sys::Array::new();
                    layers.set(0, layer.as_ref().clone());

                    let state = self.clone();
                    let display_ = display.clone();
                    let closure =
                        Closure::once(move |_| state.update(Msg::DisplayPresenting(display_)));
                    display
                        .request_present(&layers)
                        .unwrap_throw()
                        .then(&closure);
                    closure.forget();

                    model.vr_status = VrStatus::RequestedPresentation(display.clone());
                }
            }
            Msg::KeyDown(k) => {
                model.keys.insert(k.to_lowercase());

                match &k as &str {
                    "Enter" => {
                        model.world.add_sand(1);
                        model.renderer.set_world_tex(model.world.to_color_array());
                    }
                    "k" => {
                        model.world.add_sand(1_000);
                        model.renderer.set_world_tex(model.world.to_color_array());
                    }
                    "m" => {
                        model.world.add_sand(1_000_000);
                        model.renderer.set_world_tex(model.world.to_color_array());
                    }
                    "o" => {
                        let mut isom = nalgebra::Isometry3::translation(
                            -((WORLD_SIZE / 2) as f32 + 0.5),
                            -((WORLD_SIZE / 2) as f32 + 0.5),
                            -((WORLD_SIZE / 2) as f32 + 0.5),
                        );
                        isom.append_rotation_mut(&model.camera.rotation);
                        model.camera = isom;
                    }
                    _ => {}
                }
            }
            Msg::KeyUp(k) => {
                model.keys.remove(&k.to_lowercase());
            }
            Msg::MouseMove([x, y]) => {
                if model.document.pointer_lock_element().is_some() {
                    model
                        .camera
                        .append_rotation_mut(&nalgebra::UnitQuaternion::new(
                            nalgebra::Vector3::new(y as f32 * 3e-3, x as f32 * 3e-3, 0.),
                        ));
                }
            }

            Msg::GotVRDisplays(vr_displays) => {
                if vr_displays.length() == 0 {
                    model.vr_status = VrStatus::NotFound;
                } else {
                    model.vr_status = VrStatus::Known(vr_displays.get(0).dyn_into().unwrap_throw());
                }
            }
            Msg::DisplayPresenting(display) => model.vr_status = VrStatus::Presenting(display),
        }
    }

    fn frame(&self, timestamp: f64) {
        let model: &mut Model = &mut self.0.borrow_mut();

        if let VrStatus::Presenting(display) = &model.vr_status {
            display
                .request_animation_frame(&model.animation_frame_closure)
                .unwrap_throw();
        } else {
            model
                .window
                .request_animation_frame(&model.animation_frame_closure)
                .unwrap_throw();
        }

        let brightness = model.brightness();
        if let Some(fps) = &mut model.fps {
            let dt = fps.frame(timestamp);

            model.info_box.set_inner_text(&format!(
                "{}\ntotal grains: {}\nbrightness: {}\nopacity: {}% per block",
                fps,
                model.world.total_grains(),
                brightness,
                model.opacity_slider.value(),
            ));

            {
                let mut movement_vector = nalgebra::Vector3::zeros();
                if model.keys.contains(" ") {
                    movement_vector -= nalgebra::Vector3::y();
                }
                if model.keys.contains("shift") {
                    movement_vector += nalgebra::Vector3::y();
                }
                if model.keys.contains("w") {
                    movement_vector += nalgebra::Vector3::z();
                }
                if model.keys.contains("s") {
                    movement_vector -= nalgebra::Vector3::z();
                }
                if model.keys.contains("a") {
                    movement_vector += nalgebra::Vector3::x();
                }
                if model.keys.contains("d") {
                    movement_vector -= nalgebra::Vector3::x();
                }
                model
                    .camera
                    .append_translation_mut(&nalgebra::Translation::from(
                        movement_vector * dt as f32,
                    ));
            }

            {
                let views = if let VrStatus::Presenting(display) = &model.vr_status {
                    let frame_data = web_sys::VrFrameData::new().unwrap_throw();
                    display.get_frame_data(&frame_data);

                    vec![
                        render::View {
                            camera: nalgebra::MatrixSlice4::from_slice(
                                &frame_data.left_projection_matrix().unwrap_throw(),
                            ) * nalgebra::MatrixSlice4::from_slice(
                                &frame_data.left_view_matrix().unwrap_throw(),
                            ) * model.camera.to_homogeneous(),
                            viewport_start: [0, 0],
                            viewport_size: [
                                model.canvas.width() as i32 / 2,
                                model.canvas.height() as i32,
                            ],
                        },
                        render::View {
                            camera: nalgebra::MatrixSlice4::from_slice(
                                &frame_data.right_projection_matrix().unwrap_throw(),
                            ) * nalgebra::MatrixSlice4::from_slice(
                                &frame_data.right_view_matrix().unwrap_throw(),
                            ) * model.camera.to_homogeneous(),
                            viewport_start: [model.canvas.width() as i32 / 2, 0],
                            viewport_size: [
                                model.canvas.width() as i32 / 2,
                                model.canvas.height() as i32,
                            ],
                        },
                    ]
                } else {
                    vec![render::View {
                        camera: nalgebra::Perspective3::new(
                            model.canvas.width() as f32 / model.canvas.height() as f32,
                            std::f32::consts::FRAC_PI_2,
                            0.1,
                            10.,
                        )
                        .to_homogeneous()
                            * model.camera.to_homogeneous(),
                        viewport_start: [0, 0],
                        viewport_size: [model.canvas.width() as i32, model.canvas.height() as i32],
                    }]
                };
                model.renderer.render(
                    views,
                    model.brightness(),
                    model.opacity_slider.value_as_number() as f32 * 0.01,
                    model
                        .color_sliders
                        .iter()
                        .flatten()
                        .map(|x| x.value_as_number() as f32 * 0.2),
                );

                if let VrStatus::Presenting(display) = &model.vr_status {
                    display.submit_frame();
                }
            }
        } else {
            model.fps = Some(fps::FrameCounter::new(timestamp))
        }
    }

    fn event_listener(
        &self,
        target: &web_sys::EventTarget,
        event: &str,
        msg: impl Fn(web_sys::Event) -> Msg + 'static,
    ) {
        let state = self.clone();
        let closure: Closure<dyn FnMut(web_sys::Event)> = Closure::wrap(Box::new(move |evt| {
            state.update(msg(evt));
        }));
        target
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .unwrap_throw();
        closure.forget();
    }
}

impl Model {
    fn new() -> Self {
        let window = web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let body = document.body().unwrap_throw();

        body.style().set_css_text(
            r"display: grid;
grid-gap: 10px;
background-color: #000000;
color: #FFFFFF;
padding: 10px;
grid-template-areas:
    'canvas brightness brightness brightness'
    'canvas opacity    opacity    opacity   '
    'canvas color_r1   color_g1   color_b1  '
    'canvas color_r2   color_g2   color_b2  '
    'canvas color_r3   color_g3   color_b3  '
    'canvas color_r4   color_g4   color_b4  '
    'canvas color_r5   color_g5   color_b5  '
    'info   info       info       info      ';

position: fixed;
top: 0;
left: 0;
right: 0;
bottom: 0;
",
        );

        let canvas = document
            .create_element("canvas")
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap_throw();
        canvas.set_attribute("width", "1200").unwrap_throw();
        canvas.set_attribute("height", "600").unwrap_throw();
        canvas
            .style()
            .set_property("grid-area", "canvas")
            .unwrap_throw();
        body.append_child(&canvas).unwrap_throw();

        let info_box = document
            .create_element("p")
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlParagraphElement>()
            .unwrap_throw();
        info_box
            .style()
            .set_property("grid-area", "info")
            .unwrap_throw();
        body.append_child(&info_box).unwrap_throw();

        let brightness_slider = document
            .create_element("input")
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap_throw();
        brightness_slider.set_type("range");
        brightness_slider.set_min("0");
        brightness_slider.set_max("20");
        brightness_slider.set_value("20");
        brightness_slider
            .style()
            .set_property("grid-area", "brightness")
            .unwrap_throw();
        body.append_child(&brightness_slider).unwrap_throw();

        let opacity_slider = document
            .create_element("input")
            .unwrap_throw()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap_throw();
        opacity_slider.set_type("range");
        opacity_slider.set_min("0");
        opacity_slider.set_max("100");
        opacity_slider.set_value("0");
        opacity_slider
            .style()
            .set_property("grid-area", "opacity")
            .unwrap_throw();
        body.append_child(&opacity_slider).unwrap_throw();

        let mut world = sandpile::World::default();
        world.add_sand(1);

        let mut renderer = render::Renderer::new(&canvas);
        renderer.set_world_tex(world.to_color_array());

        let camera = {
            let x = (WORLD_SIZE / 2) as f32;
            nalgebra::Isometry3::look_at_rh(
                &nalgebra::Point3::new(x + 1.499, x + 1.499, x + 2.499),
                &nalgebra::Point3::new(x + 0.5, x + 0.5, x + 0.5),
                &nalgebra::Vector3::y(),
            )
        };

        let make_slider = |identifier: &str, value: &str| {
            let slider = document
                .create_element("input")
                .unwrap_throw()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap_throw();
            slider.set_type("range");
            slider.set_min("0");
            slider.set_max("5");
            slider.set_value(value);
            slider
                .style()
                .set_property("grid-area", identifier)
                .unwrap_throw();
            body.append_child(&slider).unwrap_throw();
            slider
        };

        let color_sliders = [
            [
                make_slider("color_r1", "0"),
                make_slider("color_g1", "0"),
                make_slider("color_b1", "5"),
            ],
            [
                make_slider("color_r2", "0"),
                make_slider("color_g2", "4"),
                make_slider("color_b2", "4"),
            ],
            [
                make_slider("color_r3", "0"),
                make_slider("color_g3", "5"),
                make_slider("color_b3", "0"),
            ],
            [
                make_slider("color_r4", "4"),
                make_slider("color_g4", "4"),
                make_slider("color_b4", "0"),
            ],
            [
                make_slider("color_r5", "5"),
                make_slider("color_g5", "0"),
                make_slider("color_b5", "0"),
            ],
        ];

        Self {
            animation_frame_closure: JsValue::undefined().into(),
            fps: None,
            keys: HashSet::new(),
            vr_status: VrStatus::Searching,
            renderer,

            window,
            document,
            canvas,
            info_box,
            brightness_slider,
            opacity_slider,
            color_sliders,

            camera,
            world,
        }
    }

    fn brightness(&self) -> f32 {
        ((self.brightness_slider.value_as_number() as f32 - 22.) * 0.2).exp()
    }
}
