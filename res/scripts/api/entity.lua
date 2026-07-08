-- NOTE: These files do nothing and are never read. They serve only as API definitions.

---@diagnostic disable: missing-return

---@class Entity
---@field speed integer
---@field sub_speed integer
---@field x integer
---@field y integer
---@field solid boolean
---@field walk_over boolean
---@field layer integer
---@field frame integer
local Entity = {}

---@param self Entity
---@return boolean
function Entity:moving() end

--- Return a unique ID defined in the map editor
---@param self Entity
---@return integer
function Entity:id() end

---@param self Entity
---@param direction Direction
function Entity:walk(direction) end

---@param self Entity
---@param direction Direction
function Entity:walk_noclip(direction) end

--- Define meta properties in the meta tag in tiled and access them with this method
---@param self Entity
---@param key string
---@return any?
function Entity:meta(key) end

return Entity