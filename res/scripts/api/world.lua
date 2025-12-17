-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return

---@class World
local World = {}

---@param self World
---@return integer
function World:width() end

---@param self World
---@return integer
function World:height() end

---@param self World
---@param x0 integer
---@param y0 integer
---@param x1 integer
---@param y1 integer
---@return integer
function World:looped_distance(x0, y0, x1, y1) end

---@param self World
---@param x0 integer
---@param x1 integer
---@return integer
function World:looped_distance_x(x0, x1) end

---@param self World
---@param y0 integer
---@param y1 integer
---@return integer
function World:looped_distance_y(y0, y1) end

---@param self World
---@return boolean
function World:looping() end

---@param self World
---@return boolean
function World:looping_x() end

---@param self World
---@return boolean
function World:looping_y() end

---@param self World
---@param x integer
---@param y integer
---@return integer
function World:wrap(x, y) end

---@param self World
---@param x integer
---@param y integer
---@return integer
function World:wrap_tile(x, y) end

---@param self World
---@param x integer
---@param y integer
---@param layer integer
---@return boolean
function World:collide(x, y, layer) end

---@param self World
---@param x integer
---@param y integer
---@param layer integer
---@return boolean
function World:collide_tile(x, y, layer) end

---@param self World
---@param sound string
---@param speed number
---@param volume number
function World:play(sound, speed, volume) end

--- Queue a world transition, x and y are in tile coordinates.
---@param path string
---@param transition Transition
---@param x integer
---@param y integer
function World:change_map(path, transition, x, y) end

return World