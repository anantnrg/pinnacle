// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Mutex;

use smithay::{
    backend::renderer::{
        element::{
            self, surface::WaylandSurfaceRenderElement, texture::TextureBuffer, AsRenderElements,
            Wrap,
        },
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{
        layer_map_for_output,
        space::{SpaceRenderElements, SurfaceTree},
        Space,
    },
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    render_elements,
    utils::{IsAlive, Logical, Physical, Point, Scale},
    wayland::{compositor, input_method::InputMethodHandle, shell::wlr_layer},
};

use crate::{state::WithState, tag::Tag, window::WindowElement};

use self::pointer::{PointerElement, PointerRenderElement};

pub mod pointer;

render_elements! {
    pub CustomRenderElements<R> where R: ImportAll + ImportMem;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>,
}

render_elements! {
    pub OutputRenderElements<R, E> where R: ImportAll + ImportMem;
    Space=SpaceRenderElements<R, E>,
    Window=Wrap<E>,
    Custom=CustomRenderElements<R>,
    // TODO: preview
}

impl<R> AsRenderElements<R> for WindowElement
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Texture + 'static,
{
    type RenderElement = WaylandSurfaceRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        // let window_bbox = self.bbox();
        match self {
            WindowElement::Wayland(window) => {
                window.render_elements(renderer, location, scale, alpha)
            }
            WindowElement::X11(surface) => {
                surface.render_elements(renderer, location, scale, alpha)
            }
        }
        .into_iter()
        .map(C::from)
        .collect()
    }
}

struct LayerRenderElements<R> {
    background: Vec<WaylandSurfaceRenderElement<R>>,
    bottom: Vec<WaylandSurfaceRenderElement<R>>,
    top: Vec<WaylandSurfaceRenderElement<R>>,
    overlay: Vec<WaylandSurfaceRenderElement<R>>,
}

fn layer_render_elements<R>(output: &Output, renderer: &mut R) -> LayerRenderElements<R>
where
    R: Renderer + ImportAll,
    <R as Renderer>::TextureId: 'static,
{
    let layer_map = layer_map_for_output(output);
    let mut overlay = vec![];
    let mut top = vec![];
    let mut bottom = vec![];
    let mut background = vec![];

    let layer_elements = layer_map
        .layers()
        .filter_map(|surface| {
            layer_map
                .layer_geometry(surface)
                .map(|geo| (surface, geo.loc))
        })
        .map(|(surface, loc)| {
            let render_elements = surface.render_elements::<WaylandSurfaceRenderElement<R>>(
                renderer,
                loc.to_physical(1),
                Scale::from(1.0),
                1.0,
            );
            (surface.layer(), render_elements)
        });

    for (layer, elements) in layer_elements {
        match layer {
            wlr_layer::Layer::Background => background.extend(elements),
            wlr_layer::Layer::Bottom => bottom.extend(elements),
            wlr_layer::Layer::Top => top.extend(elements),
            wlr_layer::Layer::Overlay => overlay.extend(elements),
        }
    }

    LayerRenderElements {
        background,
        bottom,
        top,
        overlay,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_render_elements<R, T>(
    space: &Space<WindowElement>,
    windows: &[WindowElement],
    pointer_location: Point<f64, Logical>,
    cursor_status: &mut CursorImageStatus,
    dnd_icon: Option<&WlSurface>,
    focus_stack: &[WindowElement],
    renderer: &mut R,
    output: &Output,
    input_method: &InputMethodHandle,
    pointer_element: &mut PointerElement<T>,
    pointer_image: Option<&TextureBuffer<T>>,
) -> Vec<OutputRenderElements<R, WaylandSurfaceRenderElement<R>>>
where
    R: Renderer<TextureId = T> + ImportAll + ImportMem,
    <R as Renderer>::TextureId: 'static,
    T: Texture + Clone,
{
    let output_geometry = space
        .output_geometry(output)
        .expect("called output_geometry on an unmapped output");
    let scale = Scale::from(output.current_scale().fractional_scale());

    let mut custom_render_elements: Vec<CustomRenderElements<_>> = Vec::new();
    // draw input method surface if any
    let rectangle = input_method.coordinates();
    let position = Point::from((
        rectangle.loc.x + rectangle.size.w,
        rectangle.loc.y + rectangle.size.h,
    ));
    input_method.with_surface(|surface| {
        custom_render_elements.extend(AsRenderElements::<R>::render_elements(
            &SurfaceTree::from_surface(surface),
            renderer,
            position.to_physical_precise_round(scale),
            scale,
            1.0,
        ));
    });

    if output_geometry.to_f64().contains(pointer_location) {
        let cursor_hotspot = if let CursorImageStatus::Surface(ref surface) = cursor_status {
            compositor::with_states(surface, |states| {
                states
                    .data_map
                    .get::<Mutex<CursorImageAttributes>>()
                    .expect("surface data map had no CursorImageAttributes")
                    .lock()
                    .expect("failed to lock mutex")
                    .hotspot
            })
        } else {
            (0, 0).into()
        };
        let cursor_pos = pointer_location - output_geometry.loc.to_f64() - cursor_hotspot.to_f64();
        let cursor_pos_scaled = cursor_pos.to_physical(scale).to_i32_round();

        // set cursor
        if let Some(pointer_image) = pointer_image {
            pointer_element.set_texture(pointer_image.clone());
        }

        // draw the cursor as relevant and
        // reset the cursor if the surface is no longer alive
        if let CursorImageStatus::Surface(surface) = &*cursor_status {
            if !surface.alive() {
                *cursor_status = CursorImageStatus::Default;
            }
        }

        pointer_element.set_status(cursor_status.clone());

        custom_render_elements.extend(pointer_element.render_elements(
            renderer,
            cursor_pos_scaled,
            scale,
            1.0,
        ));

        if let Some(dnd_icon) = dnd_icon {
            custom_render_elements.extend(AsRenderElements::render_elements(
                &smithay::desktop::space::SurfaceTree::from_surface(dnd_icon),
                renderer,
                cursor_pos_scaled,
                scale,
                1.0,
            ));
        }
    }

    let output_render_elements = {
        let top_fullscreen_window = focus_stack.iter().rev().find(|win| {
            win.with_state(|state| {
                // TODO: for wayland windows, check if current state has xdg_toplevel fullscreen
                let is_wayland_actually_fullscreen = {
                    if let WindowElement::Wayland(window) = win {
                        window
                            .toplevel()
                            .current_state()
                            .states
                            .contains(xdg_toplevel::State::Fullscreen)
                    } else {
                        true
                    }
                };
                state.fullscreen_or_maximized.is_fullscreen()
                    && state.tags.iter().any(|tag| tag.active())
                    && is_wayland_actually_fullscreen
            })
        });

        // If fullscreen windows exist, render only the topmost one
        // TODO: wait until the fullscreen window has committed, this will stop flickering
        if let Some(window) = top_fullscreen_window {
            let mut output_render_elements =
                Vec::<OutputRenderElements<_, WaylandSurfaceRenderElement<_>>>::new();

            let window_render_elements: Vec<WaylandSurfaceRenderElement<_>> =
                window.render_elements(renderer, (0, 0).into(), scale, 1.0);

            output_render_elements.extend(
                custom_render_elements
                    .into_iter()
                    .map(OutputRenderElements::from),
            );

            output_render_elements.extend(
                window_render_elements
                    .into_iter()
                    .map(|elem| OutputRenderElements::Window(element::Wrap::from(elem))),
            );

            output_render_elements
        } else {
            let LayerRenderElements {
                background,
                bottom,
                top,
                overlay,
            } = layer_render_elements(output, renderer);

            let window_render_elements: Vec<WaylandSurfaceRenderElement<R>> =
                Tag::tag_render_elements(windows, space, renderer);

            let mut output_render_elements =
                Vec::<OutputRenderElements<R, WaylandSurfaceRenderElement<R>>>::new();

            // let space_render_elements =
            //     smithay::desktop::space::space_render_elements(renderer, [space], output, 1.0)
            //         .expect("failed to get space_render_elements");

            // Elements render from top to bottom

            output_render_elements.extend(
                custom_render_elements
                    .into_iter()
                    .map(OutputRenderElements::from),
            );

            output_render_elements.extend(
                overlay
                    .into_iter()
                    .chain(top)
                    .chain(window_render_elements)
                    .chain(bottom)
                    .chain(background)
                    .map(CustomRenderElements::from)
                    .map(OutputRenderElements::from),
            );

            // output_render_elements.extend(
            //     space_render_elements
            //         .into_iter()
            //         .map(OutputRenderElements::from),
            // );

            output_render_elements
        }
    };

    output_render_elements
}
