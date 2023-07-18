// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::atomic::AtomicU32;

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    wayland::{
        compositor::{Blocker, BlockerState},
        seat::WaylandFocus,
    },
};

use crate::{
    backend::Backend,
    state::{State, WithState},
};

use self::window_state::Float;

pub mod window_state;

impl<B: Backend> State<B> {
    /// Returns the [Window] associated with a given [WlSurface].
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .find(|&win| win.toplevel().wl_surface() == surface)
                    .cloned()
            })
    }
}

/// Toggle a window's floating status.
pub fn toggle_floating<B: Backend>(state: &mut State<B>, window: &Window) {
    window.with_state(|window_state| {
        match window_state.floating {
            Float::Tiled(prev_loc_and_size) => {
                if let Some((prev_loc, prev_size)) = prev_loc_and_size {
                    window.toplevel().with_pending_state(|state| {
                        state.size = Some(prev_size);
                    });

                    window.toplevel().send_pending_configure();

                    state.space.map_element(window.clone(), prev_loc, false);
                    // TODO: should it activate?
                }

                window_state.floating = Float::Floating;
                window.toplevel().with_pending_state(|tl_state| {
                    tl_state.states.unset(xdg_toplevel::State::TiledTop);
                    tl_state.states.unset(xdg_toplevel::State::TiledBottom);
                    tl_state.states.unset(xdg_toplevel::State::TiledLeft);
                    tl_state.states.unset(xdg_toplevel::State::TiledRight);
                });
            }
            Float::Floating => {
                window_state.floating = Float::Tiled(Some((
                    // We get the location this way because window.geometry().loc
                    // doesn't seem to be the actual location
                    state.space.element_location(window).unwrap(),
                    window.geometry().size,
                )));
                window.toplevel().with_pending_state(|tl_state| {
                    tl_state.states.set(xdg_toplevel::State::TiledTop);
                    tl_state.states.set(xdg_toplevel::State::TiledBottom);
                    tl_state.states.set(xdg_toplevel::State::TiledLeft);
                    tl_state.states.set(xdg_toplevel::State::TiledRight);
                });
            }
        }
    });

    let output = state.focus_state.focused_output.clone().unwrap();
    state.re_layout(&output);

    let render = output.with_state(|op_state| {
        state
            .windows
            .iter()
            .cloned()
            .filter(|win| {
                win.with_state(|win_state| {
                    if win_state.floating.is_floating() {
                        return true;
                    }
                    for tag in win_state.tags.iter() {
                        if op_state.focused_tags().any(|tg| tg == tag) {
                            return true;
                        }
                    }
                    false
                })
            })
            .collect::<Vec<_>>()
    });

    let clone = window.clone();
    state.loop_handle.insert_idle(move |data| {
        crate::state::schedule_on_commit(data, render, move |dt| {
            dt.state.space.raise_element(&clone, true);
        });
    });
}

pub struct WindowBlocker;
pub static BLOCKER_COUNTER: AtomicU32 = AtomicU32::new(0);

impl Blocker for WindowBlocker {
    fn state(&self) -> BlockerState {
        if BLOCKER_COUNTER.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            BlockerState::Pending
        } else {
            BlockerState::Released
        }
    }
}
