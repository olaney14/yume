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

--- Return the minimum distance between two points in a looping world
---@param self World
---@param x0 integer
---@param y0 integer
---@param x1 integer
---@param y1 integer
---@return integer
function World:looped_distance(x0, y0, x1, y1) end

--- Return the minimum distance between two x coordinates in a looping world
---@param self World
---@param x0 integer
---@param x1 integer
---@return integer
function World:looped_distance_x(x0, x1) end

--- Return the minimum distance between two y coordinates in a looping world
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

--- Return x, y wrapped if out of bounds
---@param self World
---@param x integer
---@param y integer
---@return integer
function World:wrap(x, y) end

--- Return x, y (tile coordinates) wrapped if out of bounds
---@param self World
---@param x integer
---@param y integer
---@return integer
function World:wrap_tile(x, y) end

--- Return if there is any collision at x, y
---@param self World
---@param x integer
---@param y integer
---@param layer integer
---@return boolean
function World:collide(x, y, layer) end

--- Return if there is any collision at x, y (tile coordinates)
---@param self World
---@param x integer
---@param y integer
---@param layer integer
---@return boolean
function World:collide_tile(x, y, layer) end

--- Play a sound effect at speed and volume
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

--- Return a random number on [0.0,1.0) unique to the dream session
---@return number
function World:session_random() end

--- Return a random number on [0.0,1.0) changed on each level transition
---@return number
function World:level_random() end

--- Give the player an effect if they do not already have it
---@param effect string
function World:give_effect(effect) end

return World