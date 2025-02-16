-- SPDX-License-Identifier: GPL-3.0-or-later

---Window management.
---
---This module helps you deal with setting windows to fullscreen and maximized, setting their size,
---moving them between tags, and various other actions.
---@class WindowModule
local window_module = {
    ---Window rules.
    rules = require("window_rules"),
}

---A window object.
---
---This is a representation of an application window to the config process.
---
---You can retrieve windows through the various `get` function in the `WindowModule`.
---@classmod
---@class Window
---@field private _id integer The internal id of this window
local window = {}

---@param window_id WindowId
---@return Window
local function create_window(window_id)
    ---@type Window
    local w = { _id = window_id }
    -- Copy functions over
    for k, v in pairs(window) do
        w[k] = v
    end

    return w
end

---Get this window's unique id.
---
---***You will probably not need to use this.***
---@return WindowId
function window:id()
    return self._id
end

---Set this window's size.
---
---See `WindowModule.set_size` for examples.
---
---@param size { w: integer?, h: integer? }
---@see WindowModule.set_size — The corresponding module function
function window:set_size(size)
    window_module.set_size(self, size)
end

---Move this window to a tag, removing all other ones.
---
---See `WindowModule.move_to_tag` for examples.
---
---@param t TagConstructor
---@see WindowModule.move_to_tag — The corresponding module function
function window:move_to_tag(t)
    window_module.move_to_tag(self, t)
end

---Toggle the specified tag for this window.
---
---Note: toggling off all tags currently makes a window not respond to layouting.
---
---See `WindowModule.toggle_tag` for examples.
---@param t TagConstructor
---@see WindowModule.toggle_tag — The corresponding module function
function window:toggle_tag(t)
    window_module.toggle_tag(self, t)
end

---Close this window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---See `WindowModule.close` for examples.
---@see WindowModule.close — The corresponding module function
function window:close()
    window_module.close(self)
end

---Get this window's size.
---
---See `WindowModule.size` for examples.
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see WindowModule.size — The corresponding module function
function window:size()
    return window_module.size(self)
end

---Get this window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The location of this window is relative inside the sheet.
---
---If you don't set the location of your monitors, they will start at (0, 0)
---and extend rightward with their tops aligned.
---
---See `WindowModule.loc` for examples.
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
---@see WindowModule.loc — The corresponding module function
function window:loc()
    return window_module.loc(self)
end

---Get this window's class. This is usually the name of the application.
---
---See `WindowModule.class` for examples.
---@return string|nil class This window's class, or nil if it doesn't exist.
---@see WindowModule.class — The corresponding module function
function window:class()
    return window_module.class(self)
end

---Get this window's title.
---
---See `WindowModule.title` for examples.
---@return string|nil title This window's title, or nil if it doesn't exist.
---@see WindowModule.title — The corresponding module function
function window:title()
    return window_module.title(self)
end

---Get this window's floating status.
---@return boolean|nil
---@see WindowModule.floating — The corresponding module function
function window:floating()
    return window_module.floating(self)
end

---Get this window's fullscreen status.
---@return boolean|nil
---@see WindowModule.fullscreen — The corresponding module function
function window:fullscreen()
    return window_module.fullscreen(self)
end

---Get this window's maximized status.
---@return boolean|nil
---@see WindowModule.maximized — The corresponding module function
function window:maximized()
    return window_module.maximized(self)
end

---Toggle this window's floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
function window:toggle_floating()
    window_module.toggle_floating(self)
end

---Toggle this window's fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
function window:toggle_fullscreen()
    window_module.toggle_fullscreen(self)
end

---Toggle this window's maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
function window:toggle_maximized()
    window_module.toggle_maximized(self)
end

---Get whether or not this window is focused.
---
---See `WindowModule.focused` for examples.
---@return boolean|nil
---@see WindowModule.focused — The corresponding module function
function window:focused()
    return window_module.focused(self)
end

-------------------------------------------------------------------

---Get all windows with the specified class (usually the name of the application).
---@param class string The class. For example, Alacritty's class is "Alacritty".
---@return Window[]
function window_module.get_by_class(class)
    local windows = window_module.get_all()

    ---@type Window[]
    local windows_ret = {}
    for _, w in pairs(windows) do
        if w:class() == class then
            table.insert(windows_ret, w)
        end
    end

    return windows_ret
end

---Get all windows with the specified title.
---@param title string The title.
---@return Window[]
function window_module.get_by_title(title)
    local windows = window_module.get_all()

    ---@type Window[]
    local windows_ret = {}
    for _, w in pairs(windows) do
        if w:title() == title then
            table.insert(windows_ret, w)
        end
    end

    return windows_ret
end

---Get the currently focused window.
---@return Window|nil
function window_module.get_focused()
    -- TODO: get focused on output
    local windows = window_module.get_all()

    for _, w in pairs(windows) do
        if w:focused() then
            return w
        end
    end

    return nil
end

---Get all windows.
---@return Window[]
function window_module.get_all()
    local window_ids = Request("GetWindows").RequestResponse.response.Windows.window_ids

    ---@type Window[]
    local windows = {}

    for _, window_id in pairs(window_ids) do
        table.insert(windows, create_window(window_id))
    end

    return windows
end

---Toggle the tag with the given name and (optional) output for the specified window.
---
---@param w Window
---@param t TagConstructor
---@see Window.toggle_tag — The corresponding object method
function window_module.toggle_tag(w, t)
    local t = require("tag").get(t)

    if t then
        SendMsg({
            ToggleTagOnWindow = {
                window_id = w:id(),
                tag_id = t:id(),
            },
        })
    end
end

---Move the specified window to the tag with the given name and (optional) output.
---
---@param w Window
---@param t TagConstructor
---@see Window.move_to_tag — The corresponding object method
function window_module.move_to_tag(w, t)
    local t = require("tag").get(t)

    if t then
        SendMsg({
            MoveWindowToTag = {
                window_id = w:id(),
                tag_id = t:id(),
            },
        })
    end
end

---Toggle `win`'s floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
---@param win Window
function window_module.toggle_floating(win)
    SendMsg({
        ToggleFloating = {
            window_id = win:id(),
        },
    })
end

---Toggle `win`'s fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
---@param win Window
function window_module.toggle_fullscreen(win)
    SendMsg({
        ToggleFullscreen = {
            window_id = win:id(),
        },
    })
end

---Toggle `win`'s maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
---@param win Window
function window_module.toggle_maximized(win)
    SendMsg({
        ToggleMaximized = {
            window_id = win:id(),
        },
    })
end

---Set the specified window's size.
---
---### Examples
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    window.set_size(win, { w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---    window.set_size(win, { h = 300 })          -- keep the window's width but make it 300 pixels tall
---    window.set_size(win, {})                   -- do absolutely nothing useful
---end
---```
---@param win Window
---@param size { w: integer?, h: integer? }
---@see Window.set_size — The corresponding object method
function window_module.set_size(win, size)
    SendMsg({
        SetWindowSize = {
            window_id = win:id(),
            width = size.w,
            height = size.h,
        },
    })
end

---Close the specified window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---### Example
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    window.close(win) -- close the currently focused window
---end
---```
---@param win Window
---@see Window.close — The corresponding object method
function window_module.close(win)
    SendMsg({
        CloseWindow = {
            window_id = win:id(),
        },
    })
end

---Get the specified window's size.
---
---### Example
---```lua
--- -- With a 4K monitor, given a focused fullscreen window `win`...
---local size = window.size(win)
--- -- ...should have size equal to `{ w = 3840, h = 2160 }`.
---```
---@param win Window
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see Window.size — The corresponding object method
function window_module.size(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local size = response.RequestResponse.response.WindowProps.size
    if size == nil then
        return nil
    else
        return {
            w = size[1],
            h = size[2],
        }
    end
end

---Get the specified window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The location of this window is relative inside the sheet.
---
---If you don't set the location of your monitors, they will start at (0, 0)
---and extend rightward with their tops aligned.
---
---### Example
---```lua
--- -- With two 1080p monitors side by side and set up as such,
--- -- if a window `win` is fullscreen on the right one...
---local loc = window.loc(win)
--- -- ...should have loc equal to `{ x = 1920, y = 0 }`.
---```
---@param win Window
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
---@see Window.loc — The corresponding object method
function window_module.loc(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local loc = response.RequestResponse.response.WindowProps.loc
    if loc == nil then
        return nil
    else
        return {
            x = loc[1],
            y = loc[2],
        }
    end
end

---Get the specified window's class. This is usually the name of the application.
---
---### Example
---```lua
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.class(win))
---end
--- -- ...should print "Alacritty".
---```
---@param win Window
---@return string|nil class This window's class, or nil if it doesn't exist.
---@see Window.class — The corresponding object method
function window_module.class(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local class = response.RequestResponse.response.WindowProps.class
    return class
end

---Get the specified window's title.
---
---### Example
---```lua
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.title(win))
---end
--- -- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---```
---@param win Window
---@return string|nil title This window's title, or nil if it doesn't exist.
---@see Window.title — The corresponding object method
function window_module.title(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local title = response.RequestResponse.response.WindowProps.title
    return title
end

---Get this window's floating status.
---@param win Window
---@return boolean|nil
---@see Window.floating — The corresponding object method
function window_module.floating(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local floating = response.RequestResponse.response.WindowProps.floating
    return floating
end

---Get this window's fullscreen status.
---@param win Window
---@return boolean|nil
---@see Window.fullscreen — The corresponding object method
function window_module.fullscreen(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom = response.RequestResponse.response.WindowProps.fullscreen_or_maximized
    return fom == "Fullscreen"
end

---Get this window's maximized status.
---@param win Window
---@return boolean|nil
---@see Window.maximized — The corresponding object method
function window_module.maximized(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom = response.RequestResponse.response.WindowProps.fullscreen_or_maximized
    return fom == "Maximized"
end

---Get whether or not this window is focused.
---
---### Example
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    print(window.focused(win)) -- Should print `true`
---end
---```
---@param win Window
---@return boolean|nil
---@see Window.focused — The corresponding object method
function window_module.focused(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local focused = response.RequestResponse.response.WindowProps.focused
    return focused
end

---Begin a window move.
---
---This will start a window move grab with the provided button on the window the pointer
---is currently hovering over. Once `button` is let go, the move will end.
---@param button MouseButton The button you want to trigger the move.
function window_module.begin_move(button)
    SendMsg({
        WindowMoveGrab = {
            button = button,
        },
    })
end

---Begin a window resize.
---
---This will start a window resize grab with the provided button on the window the
---pointer is currently hovering over. Once `button` is let go, the resize will end.
---@param button MouseButton
function window_module.begin_resize(button)
    SendMsg({
        WindowResizeGrab = {
            button = button,
        },
    })
end

return window_module
